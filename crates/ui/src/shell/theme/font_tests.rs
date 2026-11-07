use super::super::{ShellKey, ShellState};
use super::ShellTheme;
use veila_common::AppConfig;
use veila_renderer::{
    ClearColor, FrameSize, RenderScale, SoftwareBuffer, text::MAX_CONFIGURED_FONT_REQUESTS,
};

#[test]
fn requests_keep_distinct_weights_styles_and_a_bounded_number_of_families() {
    let mut raw = String::from(
        "[visuals.input]\nfont_family=' Case Font '\nfont_weight=400\n[visuals.clock]\nfont_family='case font'\nfont_weight=700\nfont_style='italic'\n",
    );
    for index in 0..64 {
        raw.push_str(&format!(
            "[[visuals.layer]]\ntext='x'\nfont_family='Family {index}'\n"
        ));
    }
    let theme = ShellTheme::from_config(&AppConfig::from_toml_str(&raw).unwrap());
    let requests = theme.font_requests();
    assert_eq!(requests.len(), MAX_CONFIGURED_FONT_REQUESTS);
    assert!(
        requests
            .iter()
            .any(|r| r.family == "Case Font" && r.weight == 400 && !r.italic)
    );
    assert!(
        requests
            .iter()
            .any(|r| r.family == "case font" && r.weight == 700 && r.italic)
    );
}

#[test]
fn font_invalidation_preserves_input_and_rebuilds_scaled_scene_caches() {
    let config =
        AppConfig::from_toml_str("[[visuals.layer]]\ntext='Layer'\nfont_family='Geom'\n").unwrap();
    let mut shell = ShellState::new(ShellTheme::from_config(&config), None, None, true);
    shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    shell.handle_key(ShellKey::Character('a'));
    shell.handle_key(ShellKey::Character('b'));
    let requests = shell.font_requests();
    for units in [120, 150, 180, 240] {
        let scale = RenderScale::from_units(units);
        let mut before = SoftwareBuffer::solid(
            scale.frame_size(FrameSize::new(640, 360)),
            ClearColor::opaque(0, 0, 0),
        )
        .unwrap();
        shell.render_at_scale(&mut before, scale);
        let regions = shell.widget_regions_at_scale(before.size(), scale);
        let revision = shell.static_scene_revision();
        shell.invalidate_fonts();
        assert!(shell.static_scene_revision() > revision);
        assert_eq!(shell.secret.expose(), "ab");
        assert_eq!(shell.font_requests(), requests);
        let mut after = SoftwareBuffer::solid(before.size(), ClearColor::opaque(0, 0, 0)).unwrap();
        shell.render_at_scale(&mut after, scale);
        assert_eq!(before.pixels(), after.pixels());
        assert_eq!(regions, shell.widget_regions_at_scale(after.size(), scale));
    }
}
