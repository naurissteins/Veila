use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use image::{RgbaImage, imageops::FilterType};
use tiny_skia::{FillRule, FilterQuality, Mask, PathBuilder, Pixmap, PixmapPaint, Transform};

use crate::{ClearColor, FrameSize, PixelBuffer, RendererError, Result, ShadowStyle};

use super::{
    icon::{AssetIcon, IconStyle, draw_icon},
    shape::{BorderStyle, CircleStyle, PillStyle, Rect, draw_circle, draw_pill},
    skia::draw_overlay,
};

const MAX_PREPARED_AVATAR_SIZE: u32 = 512;
const AVATAR_CACHE_MAGIC: &[u8; 8] = b"VEILAVA1";

#[derive(Debug, Clone)]
pub enum AvatarAsset {
    Image(Pixmap),
    Placeholder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvatarStyle {
    pub background: ClearColor,
    pub placeholder: ClearColor,
    pub radius: Option<i32>,
    pub placeholder_padding: Option<i32>,
    pub ring: Option<BorderStyle>,
    pub shadow: Option<ShadowStyle>,
}

impl AvatarStyle {
    pub const fn new(background: ClearColor, placeholder: ClearColor) -> Self {
        Self {
            background,
            placeholder,
            radius: None,
            placeholder_padding: None,
            ring: None,
            shadow: None,
        }
    }

    pub const fn with_placeholder_padding(self, placeholder_padding: i32) -> Self {
        Self {
            background: self.background,
            placeholder: self.placeholder,
            radius: self.radius,
            placeholder_padding: Some(placeholder_padding),
            ring: self.ring,
            shadow: self.shadow,
        }
    }

    pub const fn with_ring(self, ring: BorderStyle) -> Self {
        Self {
            background: self.background,
            placeholder: self.placeholder,
            radius: self.radius,
            placeholder_padding: self.placeholder_padding,
            ring: Some(ring),
            shadow: self.shadow,
        }
    }

    pub const fn with_shadow(self, shadow: ShadowStyle) -> Self {
        Self {
            background: self.background,
            placeholder: self.placeholder,
            radius: self.radius,
            placeholder_padding: self.placeholder_padding,
            ring: self.ring,
            shadow: Some(shadow),
        }
    }

    pub const fn with_radius(self, radius: i32) -> Self {
        Self {
            background: self.background,
            placeholder: self.placeholder,
            radius: Some(radius),
            placeholder_padding: self.placeholder_padding,
            ring: self.ring,
            shadow: self.shadow,
        }
    }
}

impl AvatarAsset {
    pub fn load(path: &Path) -> Result<Self> {
        if let Some(cached) = Self::load_cached(path)? {
            return Ok(cached);
        }

        let image = image::open(path)?.to_rgba8();
        let image = prepare_avatar_image(image);
        let pixmap = rgba_to_pixmap(image)?;
        let _ = store_cached_avatar(path, &pixmap);
        Ok(Self::Image(pixmap))
    }

    pub fn load_cached(path: &Path) -> Result<Option<Self>> {
        load_cached_avatar(path).map(|avatar| avatar.map(Self::Image))
    }

    pub const fn placeholder() -> Self {
        Self::Placeholder
    }

    pub fn cache_key(&self) -> String {
        match self {
            Self::Placeholder => String::from("placeholder"),
            Self::Image(image) => {
                let mut hasher = DefaultHasher::new();
                image.data().hash(&mut hasher);
                format!(
                    "image:{}x{}:{:016x}",
                    image.width(),
                    image.height(),
                    hasher.finish()
                )
            }
        }
    }

    pub fn draw(
        &self,
        buffer: &mut impl PixelBuffer,
        center_x: i32,
        top_y: i32,
        size: u32,
        style: AvatarStyle,
    ) {
        if size == 0 {
            return;
        }

        let size_i32 = size as i32;
        let radius = resolved_avatar_radius(size_i32, style.radius);
        let left = center_x - size_i32 / 2;
        if radius >= size_i32 / 2 {
            let mut circle_style = CircleStyle::new(style.background);
            if let Some(shadow) = style.shadow {
                circle_style = circle_style.with_shadow(shadow);
            }
            if let Some(ring) = style.ring {
                circle_style = circle_style.with_border(ring);
            }
            draw_circle(
                buffer,
                center_x,
                top_y + size_i32 / 2,
                size_i32 / 2,
                circle_style,
            );
        } else {
            let rect = Rect::new(left, top_y, size_i32, size_i32);
            let mut avatar_style = PillStyle::new(style.background).with_radius(radius);
            if let Some(shadow) = style.shadow {
                avatar_style = avatar_style.with_shadow(shadow);
            }
            if let Some(ring) = style.ring {
                avatar_style = avatar_style.with_border(ring);
            }
            draw_pill(buffer, rect, avatar_style);
        }

        let inset = style
            .ring
            .map(|ring| ring.thickness.max(0) * 2)
            .unwrap_or(0);
        let content_size = (size_i32 - inset * 2).max(1) as u32;
        let content_radius = (radius - inset).clamp(0, content_size as i32 / 2);
        let content_top = top_y + inset;
        let content_left = center_x - content_size as i32 / 2;

        match self {
            Self::Image(image) => draw_avatar_image(
                buffer,
                content_left,
                content_top,
                content_size,
                content_radius,
                image,
            ),
            Self::Placeholder => draw_placeholder(
                buffer,
                content_left,
                content_top,
                content_size,
                style.placeholder,
                style.placeholder_padding,
            ),
        }
    }
}

fn prepare_avatar_image(image: RgbaImage) -> RgbaImage {
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return image;
    }

    let crop_size = width.min(height);
    let crop_x = (width - crop_size) / 2;
    let crop_y = (height - crop_size) / 2;
    let cropped =
        image::imageops::crop_imm(&image, crop_x, crop_y, crop_size, crop_size).to_image();
    let target_size = crop_size.min(MAX_PREPARED_AVATAR_SIZE);

    if target_size == crop_size {
        cropped
    } else {
        image::imageops::resize(&cropped, target_size, target_size, FilterType::Lanczos3)
    }
}

fn draw_avatar_image(
    buffer: &mut impl PixelBuffer,
    left: i32,
    top: i32,
    size: u32,
    radius: i32,
    image: &Pixmap,
) {
    draw_overlay(buffer, left, top, size, size, |overlay| {
        let Some(mut mask) = Mask::new(size, size) else {
            return;
        };
        let shape = if radius >= size as i32 / 2 {
            PathBuilder::from_circle(size as f32 / 2.0, size as f32 / 2.0, size as f32 / 2.0)
        } else {
            rounded_rect_path(size as f32, size as f32, radius as f32)
        };
        let Some(shape) = shape else {
            return;
        };
        mask.fill_path(&shape, FillRule::Winding, true, Transform::identity());

        let paint = PixmapPaint {
            quality: FilterQuality::Bicubic,
            ..PixmapPaint::default()
        };
        let scale = f32::max(
            size as f32 / image.width() as f32,
            size as f32 / image.height() as f32,
        );
        let translate_x = (size as f32 - image.width() as f32 * scale) / 2.0;
        let translate_y = (size as f32 - image.height() as f32 * scale) / 2.0;
        let transform = Transform::from_row(scale, 0.0, 0.0, scale, translate_x, translate_y);

        overlay.draw_pixmap(0, 0, image.as_ref(), &paint, transform, Some(&mask));
    });
}

fn resolved_avatar_radius(size: i32, configured_radius: Option<i32>) -> i32 {
    configured_radius
        .unwrap_or(size / 2)
        .clamp(0, (size / 2).max(0))
}

fn rounded_rect_path(width: f32, height: f32, radius: f32) -> Option<tiny_skia::Path> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let right = width;
    let bottom = height;
    let radius = radius.max(0.0).min(width.min(height) / 2.0);
    let mut builder = PathBuilder::new();

    if radius <= 0.0 {
        builder.move_to(0.0, 0.0);
        builder.line_to(right, 0.0);
        builder.line_to(right, bottom);
        builder.line_to(0.0, bottom);
    } else {
        builder.move_to(radius, 0.0);
        builder.line_to(right - radius, 0.0);
        builder.quad_to(right, 0.0, right, radius);
        builder.line_to(right, bottom - radius);
        builder.quad_to(right, bottom, right - radius, bottom);
        builder.line_to(radius, bottom);
        builder.quad_to(0.0, bottom, 0.0, bottom - radius);
        builder.line_to(0.0, radius);
        builder.quad_to(0.0, 0.0, radius, 0.0);
    }

    builder.close();
    builder.finish()
}

fn draw_placeholder(
    buffer: &mut impl PixelBuffer,
    left: i32,
    top: i32,
    size: u32,
    color: ClearColor,
    placeholder_padding: Option<i32>,
) {
    draw_icon(
        buffer,
        crate::shape::Rect::new(left, top, size as i32, size as i32),
        AssetIcon::User,
        IconStyle::new(color).with_padding(style_placeholder_padding(size, placeholder_padding)),
    );
}

fn style_placeholder_padding(size: u32, configured_padding: Option<i32>) -> i32 {
    configured_padding
        .unwrap_or_else(|| (size as i32 / 10).clamp(6, 14))
        .clamp(0, size as i32 / 3)
}

fn rgba_to_pixmap(image: RgbaImage) -> Result<Pixmap> {
    let width = image.width();
    let height = image.height();
    let size = tiny_skia::IntSize::from_wh(width, height).ok_or(
        RendererError::InvalidFrameSize(FrameSize::new(width, height)),
    )?;
    let mut data = image.into_raw();
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3];
        pixel[0] = premultiply(pixel[0], alpha);
        pixel[1] = premultiply(pixel[1], alpha);
        pixel[2] = premultiply(pixel[2], alpha);
    }
    Pixmap::from_vec(data, size).ok_or(RendererError::InvalidFrameSize(FrameSize::new(
        width, height,
    )))
}

fn load_cached_avatar(path: &Path) -> Result<Option<Pixmap>> {
    load_cached_avatar_at(path, None)
}

fn load_cached_avatar_at(path: &Path, cache_home: Option<&Path>) -> Result<Option<Pixmap>> {
    let cache_path = avatar_cache_path(path, cache_home)?;
    let Ok(mut file) = fs::File::open(&cache_path) else {
        return Ok(None);
    };

    let mut header = [0u8; 16];
    file.read_exact(&mut header)?;
    if &header[..8] != AVATAR_CACHE_MAGIC {
        return Ok(None);
    }

    let width = u32::from_le_bytes(header[8..12].try_into().expect("width slice"));
    let height = u32::from_le_bytes(header[12..16].try_into().expect("height slice"));
    let size = tiny_skia::IntSize::from_wh(width, height).ok_or(
        RendererError::InvalidFrameSize(FrameSize::new(width, height)),
    )?;
    let Some(byte_len) = FrameSize::new(width, height).byte_len() else {
        return Err(RendererError::InvalidFrameSize(FrameSize::new(
            width, height,
        )));
    };

    let mut data = vec![0; byte_len];
    file.read_exact(&mut data)?;
    Pixmap::from_vec(data, size)
        .ok_or(RendererError::InvalidFrameSize(FrameSize::new(
            width, height,
        )))
        .map(Some)
}

fn store_cached_avatar(path: &Path, pixmap: &Pixmap) -> Result<()> {
    store_cached_avatar_at(path, pixmap, None)
}

fn store_cached_avatar_at(path: &Path, pixmap: &Pixmap, cache_home: Option<&Path>) -> Result<()> {
    let cache_path = avatar_cache_path(path, cache_home)?;
    let Some(cache_dir) = cache_path.parent() else {
        return Err(RendererError::Io(std::io::Error::other(
            "avatar cache path has no parent",
        )));
    };
    fs::create_dir_all(cache_dir)?;

    let temp_path = cache_dir.join(format!(
        ".{}.tmp",
        cache_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("avatar")
    ));
    let mut file = fs::File::create(&temp_path)?;
    file.write_all(AVATAR_CACHE_MAGIC)?;
    file.write_all(&pixmap.width().to_le_bytes())?;
    file.write_all(&pixmap.height().to_le_bytes())?;
    file.write_all(pixmap.data())?;
    file.flush()?;
    fs::rename(&temp_path, &cache_path)?;

    Ok(())
}

fn avatar_cache_path(path: &Path, cache_home: Option<&Path>) -> Result<PathBuf> {
    let metadata = fs::metadata(path)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let key = stable_hash(format!(
        "{}:{}:{}:{}",
        path.display(),
        metadata.len(),
        modified,
        MAX_PREPARED_AVATAR_SIZE,
    ));

    Ok(avatar_cache_root(cache_home)?.join(format!("{key:016x}.rgba")))
}

fn avatar_cache_root(cache_home: Option<&Path>) -> Result<PathBuf> {
    let base = cache_home
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .ok_or_else(|| {
            RendererError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "failed to resolve XDG cache directory",
            ))
        })?;

    Ok(base.join("veila").join("avatars"))
}

fn stable_hash(input: String) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;

    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
}

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{
        AvatarAsset, AvatarStyle, MAX_PREPARED_AVATAR_SIZE, load_cached_avatar_at,
        prepare_avatar_image, resolved_avatar_radius, rgba_to_pixmap, store_cached_avatar_at,
        style_placeholder_padding,
    };
    use crate::{ClearColor, FrameSize, SoftwareBuffer, shape::BorderStyle};

    #[test]
    fn converts_rgba_image_to_pixmap() {
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([120, 80, 40, 255]));
        let pixmap = rgba_to_pixmap(image).expect("pixmap");

        assert_eq!(pixmap.data(), &[120, 80, 40, 255]);
    }

    #[test]
    fn prepares_avatar_images_as_bounded_square_sources() {
        let image = RgbaImage::from_pixel(1600, 1200, Rgba([120, 80, 40, 255]));
        let prepared = prepare_avatar_image(image);

        assert_eq!(prepared.width(), MAX_PREPARED_AVATAR_SIZE);
        assert_eq!(prepared.height(), MAX_PREPARED_AVATAR_SIZE);
    }

    #[test]
    fn prepares_small_avatar_images_without_upscaling() {
        let image = RgbaImage::from_pixel(320, 240, Rgba([120, 80, 40, 255]));
        let prepared = prepare_avatar_image(image);

        assert_eq!(prepared.width(), 240);
        assert_eq!(prepared.height(), 240);
    }

    #[test]
    fn round_trips_cached_avatar_pixmap() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("veila-avatar-cache-test-{unique}"));
        std::fs::create_dir_all(&root).expect("cache root");

        let avatar_path = root.join("avatar.png");
        std::fs::write(&avatar_path, b"stub").expect("avatar file");
        let image = RgbaImage::from_pixel(2, 2, Rgba([120, 80, 40, 255]));
        let pixmap = rgba_to_pixmap(image).expect("pixmap");

        store_cached_avatar_at(&avatar_path, &pixmap, Some(&root)).expect("store");
        let cached = load_cached_avatar_at(&avatar_path, Some(&root))
            .expect("load")
            .expect("cached avatar");

        assert_eq!(cached.width(), pixmap.width());
        assert_eq!(cached.height(), pixmap.height());
        assert_eq!(cached.data(), pixmap.data());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn draws_placeholder_avatar() {
        let mut buffer = SoftwareBuffer::new(FrameSize::new(160, 160)).expect("buffer");
        AvatarAsset::placeholder().draw(
            &mut buffer,
            80,
            20,
            96,
            AvatarStyle::new(
                ClearColor::rgba(255, 255, 255, 36),
                ClearColor::opaque(240, 244, 250),
            )
            .with_ring(BorderStyle::new(ClearColor::rgba(255, 255, 255, 72), 2)),
        );

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn placeholder_padding_uses_responsive_default_until_overridden() {
        assert_eq!(style_placeholder_padding(96, None), 9);
        assert_eq!(style_placeholder_padding(96, Some(12)), 12);
        assert_eq!(style_placeholder_padding(96, Some(80)), 32);
    }

    #[test]
    fn avatar_radius_defaults_to_circle_and_clamps_to_half_size() {
        assert_eq!(resolved_avatar_radius(96, None), 48);
        assert_eq!(resolved_avatar_radius(96, Some(18)), 18);
        assert_eq!(resolved_avatar_radius(96, Some(80)), 48);
        assert_eq!(resolved_avatar_radius(96, Some(-4)), 0);
    }
}
