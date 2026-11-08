use std::path::Path;
use veila_renderer::{
    ClearColor, FrameSize, RenderScale, SoftwareBuffer,
    background::{
        BackgroundTreatment, GeneratedBackground, load_cached_generated_render_variant,
        load_cached_render_variant,
    },
};
use veila_ui::StaticBackdrops;

pub(super) struct BackdropJob {
    backdrops: StaticBackdrops,
    sizes: Vec<(FrameSize, RenderScale)>,
}

#[cfg(test)]
mod tests;

impl BackdropJob {
    pub(super) fn new(backdrops: StaticBackdrops, sizes: Vec<(FrameSize, RenderScale)>) -> Self {
        Self { backdrops, sizes }
    }

    pub(super) fn needed(&self) -> bool {
        !self.sizes.is_empty() && self.backdrops.variant().is_some()
    }

    pub(super) fn store_file(
        self,
        path: &Path,
        fallback: ClearColor,
        treatment: BackgroundTreatment,
        buffers: Vec<(FrameSize, SoftwareBuffer)>,
    ) {
        self.store(buffers, |scale, buffer| {
            let Some(variant) = self.backdrops.variant_at_scale(scale) else {
                return Ok(());
            };
            if load_cached_render_variant(path, fallback, buffer.size(), treatment, &variant)?
                .is_none()
            {
                self.backdrops
                    .store_file(path, fallback, treatment, scale, buffer)?;
            }
            Ok(())
        });
    }

    pub(super) fn store_generated(
        self,
        generated: GeneratedBackground,
        treatment: BackgroundTreatment,
        buffers: Vec<(FrameSize, SoftwareBuffer)>,
    ) {
        self.store(buffers, |scale, buffer| {
            let Some(variant) = self.backdrops.variant_at_scale(scale) else {
                return Ok(());
            };
            if load_cached_generated_render_variant(generated, buffer.size(), treatment, &variant)?
                .is_none()
            {
                self.backdrops
                    .store_generated(generated, treatment, scale, buffer)?;
            }
            Ok(())
        });
    }

    fn store(
        &self,
        buffers: Vec<(FrameSize, SoftwareBuffer)>,
        mut write: impl FnMut(RenderScale, &mut SoftwareBuffer) -> veila_renderer::Result<()>,
    ) {
        if !self.needed() {
            return;
        }
        for (size, buffer) in buffers {
            let mut scales = Vec::new();
            for (candidate, scale) in &self.sizes {
                if *candidate == size && !scales.contains(scale) {
                    scales.push(*scale);
                }
            }
            let count = scales.len();
            let mut base = Some(buffer);
            for (index, scale) in scales.into_iter().enumerate() {
                // move the final base buffer, only distinct scales sharing a size need a copy
                let buffer = if index + 1 == count {
                    base.take()
                } else {
                    base.clone()
                };
                if let Some(mut buffer) = buffer
                    && let Err(error) = write(scale, &mut buffer)
                {
                    tracing::debug!(%error, "failed to warm backdrop cache");
                }
            }
        }
        tracing::debug!("automatic backdrop cache write-back finished");
    }
}
