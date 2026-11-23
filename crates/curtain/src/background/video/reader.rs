#[cfg(feature = "video-prototype")]
use std::{
    fs::OpenOptions,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
};
use std::{
    io::{self, Read},
    os::fd::AsFd,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    thread::{self, Thread},
    time::{Duration, Instant},
};

use calloop::channel::Sender;
use nix::poll::{PollFd, PollFlags, poll};
#[cfg(feature = "video-prototype")]
use nix::{fcntl::OFlag, unistd::geteuid};
#[cfg(feature = "video-prototype")]
use veila_renderer::FrameSize;
use veila_renderer::{SoftwareBuffer, background::BackgroundTreatment};
use veila_ui::StaticBackdrops;

use super::{super::BackgroundEvent, VideoFrame, decoder::Decoder, geometry::VideoGeometry};

const FRAME_PERIOD: Duration = Duration::from_millis(100);
const READ_TIMEOUT: Duration = Duration::from_secs(3);
const RUNNING: u8 = 0;
const PAUSED: u8 = 1;
const STOPPED: u8 = 2;

#[derive(Debug)]
pub(super) struct ReaderControl(AtomicU8);

impl ReaderControl {
    pub(super) fn stopped(&self) -> bool {
        self.0.load(Ordering::Acquire) == STOPPED
    }

    pub(super) fn wait_if_paused(&self) -> io::Result<bool> {
        let mut waited = false;
        loop {
            match self.0.load(Ordering::Acquire) {
                RUNNING => return Ok(waited),
                PAUSED => {
                    waited = true;
                    thread::park();
                }
                _ => return Err(io::Error::other("video reader stopped")),
            }
        }
    }
}

pub(super) enum VideoSource {
    Clip(PathBuf),
    #[cfg(feature = "video-prototype")]
    Fifo(PathBuf),
}

#[derive(Debug)]
pub(crate) struct VideoUpdate {
    pub(super) frame: Option<VideoFrame>,
    reader: Arc<ReaderControl>,
    _receipt: Option<SyncSender<()>>,
}

pub(super) struct VideoReader {
    control: Arc<ReaderControl>,
    worker: Thread,
}

impl VideoReader {
    #[cfg(test)]
    pub(super) fn test_handle() -> Self {
        Self {
            control: Arc::new(ReaderControl(AtomicU8::new(RUNNING))),
            worker: thread::current(),
        }
    }

    pub(super) fn spawn(
        source: VideoSource,
        geometry: VideoGeometry,
        treatment: BackgroundTreatment,
        backdrops: StaticBackdrops,
        events: Sender<BackgroundEvent>,
        paused: bool,
    ) -> io::Result<Self> {
        let control = Arc::new(ReaderControl(AtomicU8::new(if paused {
            PAUSED
        } else {
            RUNNING
        })));
        let reader = Arc::clone(&control);
        let worker = thread::Builder::new()
            .name("veila-video".into())
            .spawn(move || {
                video_worker(source, geometry, treatment, &backdrops, &reader, &events)
            })?;
        Ok(Self {
            control,
            worker: worker.thread().clone(),
        })
    }

    pub(super) fn owns(&self, update: &VideoUpdate) -> bool {
        Arc::ptr_eq(&self.control, &update.reader)
    }

    pub(super) fn set_paused(&self, paused: bool) {
        self.control
            .0
            .store(if paused { PAUSED } else { RUNNING }, Ordering::Release);
        if !paused {
            self.worker.unpark();
        }
    }
}

impl Drop for VideoReader {
    fn drop(&mut self) {
        self.control.0.store(STOPPED, Ordering::Release);
        self.worker.unpark();
    }
}

fn video_worker(
    source: VideoSource,
    geometry: VideoGeometry,
    treatment: BackgroundTreatment,
    backdrops: &StaticBackdrops,
    control: &Arc<ReaderControl>,
    events: &Sender<BackgroundEvent>,
) {
    if let Err(error) = feed_video(source, geometry, treatment, backdrops, control, events)
        && !control.stopped()
    {
        tracing::warn!(%error, "video prototype input ended");
    }
    if !control.stopped() {
        let _ = events.send(BackgroundEvent::Video(VideoUpdate {
            frame: None,
            reader: Arc::clone(control),
            _receipt: None,
        }));
    }
}

fn feed_video(
    source: VideoSource,
    geometry: VideoGeometry,
    treatment: BackgroundTreatment,
    backdrops: &StaticBackdrops,
    control: &Arc<ReaderControl>,
    events: &Sender<BackgroundEvent>,
) -> io::Result<()> {
    match source {
        VideoSource::Clip(path) => {
            control.wait_if_paused()?;
            let mut decoder = Decoder::spawn(&path, geometry.buffer, control)?;
            tracing::debug!("started sandboxed video decoder");
            feed_frames(
                &mut decoder.stdout()?,
                geometry,
                treatment,
                backdrops,
                control,
                events,
            )
        }
        #[cfg(feature = "video-prototype")]
        VideoSource::Fifo(path) => {
            let mut pipe = open_fifo(&path, geometry.buffer, control)?;
            feed_frames(&mut pipe, geometry, treatment, backdrops, control, events)
        }
    }
}

#[cfg(feature = "video-prototype")]
fn open_fifo(
    path: &std::path::Path,
    size: FrameSize,
    control: &ReaderControl,
) -> io::Result<std::fs::File> {
    let mut pipe = OpenOptions::new()
        .read(true)
        .custom_flags((OFlag::O_NONBLOCK | OFlag::O_NOFOLLOW | OFlag::O_NOCTTY).bits())
        .open(path)?;
    let metadata = pipe.metadata()?;
    if !metadata.file_type().is_fifo()
        || metadata.uid() != geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
    {
        return Err(io::Error::other(
            "expected a private, user-owned video FIFO",
        ));
    }
    let mut header = [0; 12];
    read_input(&mut pipe, &mut header, control, true)?;
    if header != video_header(size) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "video format or size mismatch",
        ));
    }
    Ok(pipe)
}

fn feed_frames(
    pipe: &mut (impl Read + AsFd),
    geometry: VideoGeometry,
    treatment: BackgroundTreatment,
    backdrops: &StaticBackdrops,
    control: &Arc<ReaderControl>,
    events: &Sender<BackgroundEvent>,
) -> io::Result<()> {
    let bytes = geometry
        .buffer
        .byte_len()
        .ok_or_else(|| io::Error::other("invalid video size"))?;
    while control.wait_if_paused().is_ok() {
        let started = Instant::now();
        let mut pixels = vec![0; bytes];
        read_input(pipe, &mut pixels, control, false)?;
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel[3] = 255;
        }
        let frame = SoftwareBuffer::from_argb8888_pixels(geometry.buffer, pixels)
            .map_err(io::Error::other)?;
        let frame = VideoFrame::prepare(frame, geometry, treatment, backdrops);
        control.wait_if_paused()?;
        let (receipt, consumed) = sync_channel(0);
        if events
            .send(BackgroundEvent::Video(VideoUpdate {
                frame: Some(frame),
                reader: Arc::clone(control),
                _receipt: Some(receipt),
            }))
            .is_err()
        {
            break;
        }
        // dropping the receipt after handling the event releases the one-frame queue
        let _ = consumed.recv();
        thread::sleep(FRAME_PERIOD.saturating_sub(started.elapsed()));
    }
    Ok(())
}

#[cfg(feature = "video-prototype")]
pub(super) fn video_header(size: FrameSize) -> [u8; 12] {
    let mut header = *b"VLP1\0\0\0\0\0\0\0\0";
    header[4..8].copy_from_slice(&size.width.to_le_bytes());
    header[8..12].copy_from_slice(&size.height.to_le_bytes());
    header
}

fn read_input(
    pipe: &mut (impl Read + AsFd),
    mut bytes: &mut [u8],
    control: &ReaderControl,
    mut awaiting_writer: bool,
) -> io::Result<()> {
    let mut deadline = Instant::now() + READ_TIMEOUT;
    while !bytes.is_empty() {
        if control.wait_if_paused()? {
            deadline = Instant::now() + READ_TIMEOUT;
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "video input deadline",
            ));
        }
        match pipe.read(bytes) {
            Ok(0) if awaiting_writer => thread::sleep(Duration::from_millis(20)),
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "video input closed",
                ));
            }
            Ok(count) => {
                awaiting_writer = false;
                bytes = &mut bytes[count..];
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                let mut descriptors = [PollFd::new(pipe.as_fd(), PollFlags::POLLIN)];
                match poll(&mut descriptors, 50_u16) {
                    Ok(_) | Err(nix::errno::Errno::EINTR) => {}
                    Err(error) => return Err(error.into()),
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_reader_ignores_the_stopped_readers_update() {
        let old = VideoReader::test_handle();
        let new = VideoReader::test_handle();
        let update = VideoUpdate {
            frame: None,
            reader: Arc::clone(&old.control),
            _receipt: None,
        };
        assert!(old.owns(&update));
        drop(old);
        assert!(!new.owns(&update));
    }
}
