mod fill;
mod rounded;
#[cfg(test)]
mod tests;

pub use fill::{draw_box, fill_rect, stroke_rect};
pub use rounded::{draw_circle, draw_pill};

use crate::{ClearColor, ShadowStyle};

/// Rectangle in buffer pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// Creates a rectangle.
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns whether the rectangle has drawable area.
    pub const fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    pub const fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }

    pub const fn right(self) -> i32 {
        self.x + self.width
    }

    pub const fn bottom(self) -> i32 {
        self.y + self.height
    }

    pub fn inflated(self, amount: i32) -> Self {
        if self.is_empty() {
            return self;
        }

        let amount = amount.max(0);
        Self::new(
            self.x - amount,
            self.y - amount,
            self.width + amount * 2,
            self.height + amount * 2,
        )
    }

    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }

        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Self::new(left, top, right - left, bottom - top)
    }

    pub fn clipped_to(self, width: i32, height: i32) -> Self {
        if self.is_empty() || width <= 0 || height <= 0 {
            return Self::new(0, 0, 0, 0);
        }

        let left = self.x.clamp(0, width);
        let top = self.y.clamp(0, height);
        let right = self.right().clamp(0, width);
        let bottom = self.bottom().clamp(0, height);
        Self::new(left, top, (right - left).max(0), (bottom - top).max(0))
    }
}

/// Border configuration for a filled box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderStyle {
    pub color: ClearColor,
    pub thickness: i32,
}

impl BorderStyle {
    /// Creates a border style.
    pub const fn new(color: ClearColor, thickness: i32) -> Self {
        Self { color, thickness }
    }
}

/// Fill and optional border styling for a rectangular box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxStyle {
    pub fill: ClearColor,
    pub border: Option<BorderStyle>,
}

impl BoxStyle {
    /// Creates a filled box style.
    pub const fn new(fill: ClearColor) -> Self {
        Self { fill, border: None }
    }

    /// Adds a border to a box style.
    pub const fn with_border(self, border: BorderStyle) -> Self {
        Self {
            fill: self.fill,
            border: Some(border),
        }
    }
}

/// Styling for a pill-shaped surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PillStyle {
    pub fill: ClearColor,
    pub border: Option<BorderStyle>,
    pub shadow: Option<ShadowStyle>,
    pub radius: i32,
}

impl PillStyle {
    pub const fn new(fill: ClearColor) -> Self {
        Self {
            fill,
            border: None,
            shadow: None,
            radius: i32::MAX,
        }
    }

    pub const fn with_border(self, border: BorderStyle) -> Self {
        Self {
            fill: self.fill,
            border: Some(border),
            shadow: self.shadow,
            radius: self.radius,
        }
    }

    pub const fn with_shadow(self, shadow: ShadowStyle) -> Self {
        Self {
            fill: self.fill,
            border: self.border,
            shadow: Some(shadow),
            radius: self.radius,
        }
    }

    pub const fn with_radius(self, radius: i32) -> Self {
        Self {
            fill: self.fill,
            border: self.border,
            shadow: self.shadow,
            radius,
        }
    }
}

/// Styling for a circle surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircleStyle {
    pub fill: ClearColor,
    pub border: Option<BorderStyle>,
    pub shadow: Option<ShadowStyle>,
}

impl CircleStyle {
    pub const fn new(fill: ClearColor) -> Self {
        Self {
            fill,
            border: None,
            shadow: None,
        }
    }

    pub const fn with_border(self, border: BorderStyle) -> Self {
        Self {
            fill: self.fill,
            border: Some(border),
            shadow: self.shadow,
        }
    }

    pub const fn with_shadow(self, shadow: ShadowStyle) -> Self {
        Self {
            fill: self.fill,
            border: self.border,
            shadow: Some(shadow),
        }
    }
}
