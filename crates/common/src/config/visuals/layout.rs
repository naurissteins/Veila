use serde::{Deserialize, Serialize};

use super::RgbColor;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayoutVisualConfig {
    #[serde(default)]
    pub auth_stack_offset: Option<i16>,
    #[serde(default)]
    pub header_top_offset: Option<i16>,
    #[serde(default)]
    pub identity_gap: Option<u16>,
    #[serde(default)]
    pub center_stack_order: Option<CenterStackOrder>,
    #[serde(default)]
    pub center_stack_style: Option<CenterStackStyle>,
}

impl Default for LayoutVisualConfig {
    fn default() -> Self {
        Self {
            auth_stack_offset: Some(0),
            header_top_offset: Some(-12),
            identity_gap: Some(18),
            center_stack_order: Some(CenterStackOrder::HeroAuth),
            center_stack_style: Some(CenterStackStyle::HeroAuth),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum CenterStackOrder {
    #[default]
    #[serde(rename = "hero-auth")]
    HeroAuth,
    #[serde(rename = "auth-hero")]
    AuthHero,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum CenterStackStyle {
    #[default]
    #[serde(rename = "hero-auth")]
    HeroAuth,
    #[serde(rename = "auth-hero")]
    AuthHero,
    #[serde(rename = "identity-hero-input")]
    IdentityHeroInput,
}

impl From<CenterStackOrder> for CenterStackStyle {
    fn from(value: CenterStackOrder) -> Self {
        match value {
            CenterStackOrder::HeroAuth => Self::HeroAuth,
            CenterStackOrder::AuthHero => Self::AuthHero,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaletteVisualConfig {
    #[serde(default)]
    pub foreground: Option<RgbColor>,
    #[serde(default)]
    pub muted: Option<RgbColor>,
    #[serde(default)]
    pub pending: Option<RgbColor>,
    #[serde(default)]
    pub rejected: Option<RgbColor>,
}

impl super::VisualConfig {
    pub fn auth_stack_offset(&self) -> Option<i16> {
        self.layout
            .as_ref()
            .and_then(|layout| layout.auth_stack_offset)
            .or(self.auth_stack_offset)
    }

    pub fn header_top_offset(&self) -> Option<i16> {
        self.layout
            .as_ref()
            .and_then(|layout| layout.header_top_offset)
            .or(self.header_top_offset)
    }

    pub fn identity_gap(&self) -> Option<u16> {
        self.layout
            .as_ref()
            .and_then(|layout| layout.identity_gap)
            .or(self.identity_gap)
    }

    pub fn center_stack_order(&self) -> CenterStackOrder {
        match self.center_stack_style() {
            CenterStackStyle::HeroAuth => CenterStackOrder::HeroAuth,
            CenterStackStyle::AuthHero | CenterStackStyle::IdentityHeroInput => {
                CenterStackOrder::AuthHero
            }
        }
    }

    pub fn center_stack_style(&self) -> CenterStackStyle {
        self.layout
            .as_ref()
            .and_then(|layout| layout.center_stack_style)
            .or_else(|| {
                self.layout
                    .as_ref()
                    .and_then(|layout| layout.center_stack_order.map(Into::into))
            })
            .or(self.center_stack_style)
            .or_else(|| self.center_stack_order.map(Into::into))
            .unwrap_or_default()
    }

    pub fn foreground_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.foreground)
            .unwrap_or(self.foreground)
    }

    pub fn muted_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.muted)
            .unwrap_or(self.muted)
    }

    pub fn pending_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.pending)
            .unwrap_or(self.pending)
    }

    pub fn rejected_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.rejected)
            .unwrap_or(self.rejected)
    }
}
