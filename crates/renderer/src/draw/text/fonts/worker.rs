use super::{FontRequest, MAX_CONFIGURED_FONT_REQUESTS, PreparedFonts, prepare_fonts};
use std::{
    io,
    sync::{
        Arc, Mutex,
        mpsc::{SyncSender, sync_channel},
    },
    thread,
};

type Request = (u64, Vec<FontRequest>);

pub struct FontLoader {
    pending: Arc<Mutex<Option<Request>>>,
    wake: SyncSender<()>,
}

impl FontLoader {
    pub fn new(ready: impl Fn(u64, PreparedFonts) + Send + 'static) -> io::Result<Self> {
        Self::with_prepare(ready, |requests| {
            prepare_fonts(requests, Some(std::path::Path::new("/proc/self/exe")))
        })
    }

    fn with_prepare(
        ready: impl Fn(u64, PreparedFonts) + Send + 'static,
        prepare: impl Fn(&[FontRequest]) -> PreparedFonts + Send + 'static,
    ) -> io::Result<Self> {
        let pending = Arc::new(Mutex::new(None::<Request>));
        let next = pending.clone();
        let (wake, receiver) = sync_channel(1);
        thread::Builder::new()
            .name(String::from("veila-fonts"))
            .spawn(move || {
                while receiver.recv().is_ok() {
                    let request = next.lock().ok().and_then(|mut pending| pending.take());
                    if let Some((generation, requests)) = request {
                        ready(generation, prepare(&requests));
                    }
                }
            })?;
        Ok(Self { pending, wake })
    }

    pub fn request(&self, generation: u64, mut requests: Vec<FontRequest>) {
        requests.truncate(MAX_CONFIGURED_FONT_REQUESTS);
        if let Ok(mut pending) = self.pending.lock() {
            *pending = Some((generation, requests));
        }
        // one worker coalesces reloads; dispatch never joins a font-loading thread
        let _ = self.wake.try_send(());
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
