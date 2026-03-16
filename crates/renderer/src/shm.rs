use smithay_client_toolkit::{
    reexports::client::{
        Dispatch, QueueHandle,
        protocol::{wl_buffer, wl_shm, wl_surface::WlSurface},
    },
    shm::{Shm, raw::RawPool},
};

use crate::{RendererError, Result, SoftwareBuffer};

pub fn commit_buffer<D>(
    shm: &Shm,
    queue_handle: &QueueHandle<D>,
    surface: &WlSurface,
    buffer: &SoftwareBuffer,
) -> Result<()>
where
    D: Dispatch<wl_buffer::WlBuffer, ()> + 'static,
{
    let size = buffer.size();
    if size.is_empty() {
        return Err(RendererError::EmptyFrame);
    }

    let mut pool = RawPool::new(
        size.byte_len()
            .ok_or(RendererError::InvalidFrameSize(size))?,
        shm,
    )?;
    pool.mmap().copy_from_slice(buffer.pixels());

    let wl_buffer = pool.create_buffer(
        0,
        size.width as i32,
        size.height as i32,
        (size.width * 4) as i32,
        wl_shm::Format::Argb8888,
        (),
        queue_handle,
    );
    surface.attach(Some(&wl_buffer), 0, 0);
    surface.damage_buffer(0, 0, size.width as i32, size.height as i32);
    surface.commit();
    wl_buffer.destroy();

    Ok(())
}
