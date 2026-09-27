use super::ShellTheme;

impl ShellTheme {
    pub fn font_warmup_families(&self) -> Vec<String> {
        let configured = [
            self.input_font_family.as_deref(),
            self.reveal_font_family.as_deref(),
            self.username_font_family.as_deref(),
            self.clock_font_family.as_deref(),
            self.date_font_family.as_deref(),
            self.weather_temperature_font_family.as_deref(),
            self.weather_location_font_family.as_deref(),
            self.now_playing_artist_font_family.as_deref(),
            self.now_playing_title_font_family.as_deref(),
        ];
        let mut families = Vec::new();
        for family in configured.into_iter().flatten().chain(
            self.layers
                .iter()
                .filter_map(|layer| layer.font_family.as_deref()),
        ) {
            let family = family.trim();
            if !family.is_empty()
                && !families
                    .iter()
                    .any(|existing: &String| existing.eq_ignore_ascii_case(family))
            {
                families.push(family.to_owned());
            }
        }
        families
    }
}
