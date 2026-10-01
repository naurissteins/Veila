use super::{
    GeneratedPrewarmReport, LayeredPrewarmReport, PrewarmSize, RenderedPrewarmReport,
    unique_buffer_sizes,
};
use std::time::Instant;
use veila_common::elapsed_ms;
use veila_renderer::{
    ClearColor,
    background::{
        BackgroundAsset, BackgroundTreatment, FileBackgroundPrewarm, GeneratedBackground,
        load_cached_generated_render_variant, load_cached_render_variant,
        prewarm_rendered_generated, store_cached_generated_render_variant,
        store_cached_render_variant,
    },
};
use veila_ui::ShellState;

pub(super) fn prewarm_layered_backgrounds(
    asset: &mut FileBackgroundPrewarm<'_>,
    shell: &ShellState,
    sizes: &[PrewarmSize],
) -> Option<LayeredPrewarmReport> {
    if sizes.is_empty()
        || (shell.backdrop_cache_variant().is_none()
            && shell.static_scene_cache_variant(1).is_none())
    {
        return None;
    }

    let started_at = Instant::now();
    let path = asset.path();
    let treatment = asset.treatment();
    let mut cache_hits = 0usize;
    let mut warmed_sizes = 0usize;

    for size in sizes.iter().copied() {
        let variant = shell.backdrop_cache_variant_at_scale(size.scale);
        let mut base_buffer = None;

        if let Some(variant) = &variant {
            if load_cached_render_variant(path, size.buffer, treatment, variant)
                .ok()
                .flatten()
                .is_some()
            {
                cache_hits += 1;
            } else {
                let mut buffer = asset.render(size.buffer).ok()?;
                shell.render_static_backdrops_at_scale(&mut buffer, size.scale);
                store_cached_render_variant(path, size.buffer, treatment, &buffer, variant).ok()?;
                warmed_sizes += 1;
                base_buffer = Some(buffer);
            }
        }

        if let Some(scene_variant) = shell.static_scene_cache_variant_at_scale(size.scale) {
            if load_cached_render_variant(path, size.buffer, treatment, &scene_variant)
                .ok()
                .flatten()
                .is_some()
            {
                cache_hits += 1;
                continue;
            }

            let mut buffer = match (base_buffer.take(), variant.as_deref()) {
                (Some(buffer), _) => buffer,
                (None, Some(variant)) => {
                    if let Some(buffer) =
                        load_cached_render_variant(path, size.buffer, treatment, variant)
                            .ok()
                            .flatten()
                    {
                        buffer
                    } else {
                        let mut buffer = asset.render(size.buffer).ok()?;
                        shell.render_static_backdrops_at_scale(&mut buffer, size.scale);
                        buffer
                    }
                }
                (None, None) => asset.render(size.buffer).ok()?,
            };
            shell.render_static_overlay_at_scale(&mut buffer, size.scale);
            store_cached_render_variant(path, size.buffer, treatment, &buffer, &scene_variant)
                .ok()?;
            warmed_sizes += 1;
        }
    }

    Some(LayeredPrewarmReport {
        elapsed_ms: elapsed_ms(started_at),
        probed_outputs: sizes.len(),
        cache_hits,
        warmed_sizes,
    })
}

pub(super) fn prewarm_generated_backgrounds(
    generated: GeneratedBackground,
    treatment: BackgroundTreatment,
    shell: &ShellState,
    sizes: &[PrewarmSize],
) -> Option<GeneratedPrewarmReport> {
    if sizes.is_empty() {
        return None;
    }

    let started_at = Instant::now();
    let buffer_sizes = unique_buffer_sizes(sizes);
    let summary = prewarm_rendered_generated(generated, treatment, &buffer_sizes).ok()?;
    let rendered = RenderedPrewarmReport {
        elapsed_ms: elapsed_ms(started_at),
        probed_outputs: sizes.len(),
        summary,
    };
    let layered = prewarm_generated_layered_backgrounds(generated, treatment, shell, sizes);

    Some(GeneratedPrewarmReport {
        mode: generated.mode_name(),
        rendered,
        layered,
    })
}

fn prewarm_generated_layered_backgrounds(
    generated: GeneratedBackground,
    treatment: BackgroundTreatment,
    shell: &ShellState,
    sizes: &[PrewarmSize],
) -> Option<LayeredPrewarmReport> {
    if sizes.is_empty()
        || (shell.backdrop_cache_variant().is_none()
            && shell.static_scene_cache_variant(1).is_none())
    {
        return None;
    }

    let started_at = Instant::now();
    let asset = BackgroundAsset::load(
        None,
        ClearColor::opaque(0, 0, 0),
        Some(generated),
        treatment,
    )
    .ok()?;
    let mut cache_hits = 0usize;
    let mut warmed_sizes = 0usize;

    for size in sizes.iter().copied() {
        let variant = shell.backdrop_cache_variant_at_scale(size.scale);
        let mut base_buffer = None;

        if let Some(variant) = &variant {
            if load_cached_generated_render_variant(generated, size.buffer, treatment, variant)
                .ok()
                .flatten()
                .is_some()
            {
                cache_hits += 1;
            } else {
                let mut buffer = asset.render(size.buffer).ok()?;
                shell.render_static_backdrops_at_scale(&mut buffer, size.scale);
                store_cached_generated_render_variant(
                    generated,
                    size.buffer,
                    treatment,
                    &buffer,
                    variant,
                )
                .ok()?;
                warmed_sizes += 1;
                base_buffer = Some(buffer);
            }
        }

        if let Some(scene_variant) = shell.static_scene_cache_variant_at_scale(size.scale) {
            if load_cached_generated_render_variant(
                generated,
                size.buffer,
                treatment,
                &scene_variant,
            )
            .ok()
            .flatten()
            .is_some()
            {
                cache_hits += 1;
                continue;
            }

            let mut buffer = match (base_buffer.take(), variant.as_deref()) {
                (Some(buffer), _) => buffer,
                (None, Some(variant)) => {
                    if let Some(buffer) = load_cached_generated_render_variant(
                        generated,
                        size.buffer,
                        treatment,
                        variant,
                    )
                    .ok()
                    .flatten()
                    {
                        buffer
                    } else {
                        let mut buffer = asset.render(size.buffer).ok()?;
                        shell.render_static_backdrops_at_scale(&mut buffer, size.scale);
                        buffer
                    }
                }
                (None, None) => asset.render(size.buffer).ok()?,
            };
            shell.render_static_overlay_at_scale(&mut buffer, size.scale);
            store_cached_generated_render_variant(
                generated,
                size.buffer,
                treatment,
                &buffer,
                &scene_variant,
            )
            .ok()?;
            warmed_sizes += 1;
        }
    }

    Some(LayeredPrewarmReport {
        elapsed_ms: elapsed_ms(started_at),
        probed_outputs: sizes.len(),
        cache_hits,
        warmed_sizes,
    })
}
