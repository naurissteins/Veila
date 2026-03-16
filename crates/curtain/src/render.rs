use anyhow::Result;
use smithay_client_toolkit::{
    reexports::client::{Dispatch, QueueHandle, protocol::wl_buffer, protocol::wl_shm},
    session_lock::SessionLockSurface,
    shm::{Shm, raw::RawPool},
};

const OPAQUE_BLACK: [u8; 4] = 0xFF_00_00_00_u32.to_le_bytes();

pub fn commit_blank_surface<D>(
    shm: &Shm,
    queue_handle: &QueueHandle<D>,
    surface: &SessionLockSurface,
    width: u32,
    height: u32,
) -> Result<()>
where
    D: Dispatch<wl_buffer::WlBuffer, ()> + 'static,
{
    let mut pool = RawPool::new((width * height * 4) as usize, shm)?;
    let canvas = pool.mmap();
    canvas
        .chunks_exact_mut(4)
        .for_each(|pixel| pixel.copy_from_slice(&OPAQUE_BLACK));

    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        (width * 4) as i32,
        wl_shm::Format::Argb8888,
        (),
        queue_handle,
    );
    surface.wl_surface().attach(Some(&buffer), 0, 0);
    surface
        .wl_surface()
        .damage_buffer(0, 0, width as i32, height as i32);
    surface.wl_surface().commit();
    buffer.destroy();

    Ok(())
}
