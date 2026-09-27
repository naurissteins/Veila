use crate::FrameSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderScale(u32);

impl RenderScale {
    pub const ONE: Self = Self(120);

    pub const fn from_units(units: u32) -> Self {
        Self(if units == 0 { 120 } else { units })
    }

    pub const fn from_integer(scale: u32) -> Self {
        Self(if scale == 0 {
            120
        } else {
            scale.saturating_mul(120)
        })
    }

    pub const fn units(self) -> u32 {
        self.0
    }

    pub const fn is_integer(self) -> bool {
        self.0.is_multiple_of(120)
    }

    pub fn ceil_integer(self) -> u32 {
        self.0.saturating_add(119) / 120
    }

    pub fn apply_u32(self, value: u32) -> u32 {
        (u64::from(value) * u64::from(self.0) + 60)
            .saturating_div(120)
            .min(u64::from(u32::MAX)) as u32
    }

    pub fn apply_i32(self, value: i32) -> i32 {
        let magnitude = (u64::from(value.unsigned_abs()) * u64::from(self.0) + 60) / 120;
        if value < 0 {
            -(magnitude.min(1u64 << 31) as i64) as i32
        } else {
            magnitude.min(i32::MAX as u64) as i32
        }
    }

    pub fn frame_size(self, logical: FrameSize) -> FrameSize {
        FrameSize::new(
            self.apply_u32(logical.width).max(1),
            self.apply_u32(logical.height).max(1),
        )
    }

    pub fn infer_from_mode(
        logical: FrameSize,
        mode: (i32, i32),
        integer_scale: i32,
    ) -> Option<Self> {
        let physical = (u32::try_from(mode.0).ok()?, u32::try_from(mode.1).ok()?);
        let max_units = (integer_scale.max(1) as u32).saturating_mul(120);
        [physical, (physical.1, physical.0)]
            .into_iter()
            .find_map(|(width, height)| {
                let width_units = u64::from(width)
                    .saturating_mul(120)
                    .saturating_add(u64::from(logical.width) / 2)
                    .checked_div(u64::from(logical.width))?;
                let height_units = u64::from(height)
                    .saturating_mul(120)
                    .saturating_add(u64::from(logical.height) / 2)
                    .checked_div(u64::from(logical.height))?;
                let units = u32::try_from(width_units).ok()?;
                let height_units = u32::try_from(height_units).ok()?;
                (units > 0 && units <= max_units && units.abs_diff(height_units) <= 1)
                    .then(|| Self::from_units(units))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_scale_rounds_dimensions_and_signed_theme_offsets() {
        let scale = RenderScale::from_units(180);
        assert_eq!(
            scale.frame_size(FrameSize::new(2560, 1440)),
            FrameSize::new(3840, 2160)
        );
        assert_eq!(scale.apply_i32(-3), -5);
        assert_eq!(scale.apply_i32(3), 5);
    }

    #[test]
    fn integer_scale_matches_exact_multiplication() {
        for scale in 1..=4 {
            let render = RenderScale::from_integer(scale);
            assert_eq!(render.apply_u32(379), 379 * scale);
            assert_eq!(render.apply_i32(-379), -379 * scale as i32);
        }
    }

    #[test]
    fn extreme_values_saturate() {
        let scale = RenderScale::from_units(u32::MAX);
        assert_eq!(scale.apply_u32(u32::MAX), u32::MAX);
        assert_eq!(scale.apply_i32(i32::MIN), i32::MIN);
    }

    #[test]
    fn infers_fractional_output_scale_from_current_mode() {
        let logical = FrameSize::new(1280, 720);
        assert_eq!(
            RenderScale::infer_from_mode(logical, (1920, 1080), 2),
            Some(RenderScale::from_units(180))
        );
        assert_eq!(
            RenderScale::infer_from_mode(logical, (1080, 1920), 2),
            Some(RenderScale::from_units(180))
        );
        assert!(RenderScale::infer_from_mode(logical, (1920, 900), 2).is_none());
        assert!(RenderScale::infer_from_mode(logical, (1920, 1080), 1).is_none());
    }
}
