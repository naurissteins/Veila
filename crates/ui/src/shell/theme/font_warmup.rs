use super::ShellTheme;
use veila_common::FontStyle;
use veila_renderer::text::{FontRequest, MAX_CONFIGURED_FONT_REQUESTS};

impl ShellTheme {
    pub fn font_requests(&self) -> Vec<FontRequest> {
        let configured = [
            (
                self.input.font_family.as_deref(),
                self.input.font_weight,
                self.input.font_style,
            ),
            (
                self.reveal.font_family.as_deref(),
                self.reveal.font_weight,
                self.reveal.font_style,
            ),
            (
                self.username.font_family.as_deref(),
                self.username.font_weight,
                self.username.font_style,
            ),
            (
                self.clock.font_family.as_deref(),
                self.clock.font_weight,
                self.clock.font_style,
            ),
            (
                self.date.font_family.as_deref(),
                self.date.font_weight,
                self.date.font_style,
            ),
            (
                self.weather.temperature_font_family.as_deref(),
                self.weather.temperature_font_weight,
                self.weather.temperature_font_style,
            ),
            (
                self.weather.location_font_family.as_deref(),
                self.weather.location_font_weight,
                self.weather.location_font_style,
            ),
            (
                self.now_playing.artist_font_family.as_deref(),
                self.now_playing.artist_font_weight,
                self.now_playing.artist_font_style,
            ),
            (
                self.now_playing.title_font_family.as_deref(),
                self.now_playing.title_font_weight,
                self.now_playing.title_font_style,
            ),
        ];
        let mut requests: Vec<FontRequest> = Vec::new();
        for (family, weight, style) in
            configured
                .into_iter()
                .chain(self.layers.iter().map(|layer| {
                    (
                        layer.font_family.as_deref(),
                        layer.font_weight,
                        layer.font_style,
                    )
                }))
        {
            let Some(family) = family
                .map(str::trim)
                .filter(|name| !name.is_empty() && name.len() <= 256)
            else {
                continue;
            };
            let weight = weight.unwrap_or(400);
            let italic = style == Some(FontStyle::Italic);
            if !requests.iter().any(|old| {
                old.family.eq_ignore_ascii_case(family)
                    && old.weight == weight
                    && old.italic == italic
            }) {
                if requests.len() == MAX_CONFIGURED_FONT_REQUESTS {
                    break;
                }
                requests.push(FontRequest {
                    family: family.to_owned(),
                    weight,
                    italic,
                });
            }
        }
        requests
    }

    pub fn font_warmup_families(&self) -> Vec<String> {
        let mut families = Vec::new();
        for request in self.font_requests() {
            if !families
                .iter()
                .any(|old: &String| old.eq_ignore_ascii_case(&request.family))
            {
                families.push(request.family);
            }
        }
        families
    }
}

impl super::super::ShellState {
    pub fn font_requests(&self) -> Vec<FontRequest> {
        self.theme.font_requests()
    }

    pub fn invalidate_fonts(&mut self) {
        self.scaled_render_cache.get_mut().clear();
        *self.text_layout_cache.get_mut() = Default::default();
        self.bump_static_scene_revision();
    }
}
