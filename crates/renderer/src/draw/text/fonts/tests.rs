use super::super::context::resolve_font_family_in_db;
use super::*;

#[test]
fn initial_database_contains_only_embedded_faces() {
    let fonts = bundled_fonts();
    let db = fonts.0.font_system.db();
    assert_eq!(db.faces().count(), 3);
    assert!(
        db.faces()
            .all(|face| matches!(face.source, Source::Binary(_)))
    );
    for family in ["Geom", "Google Sans Flex", "Nunito"] {
        assert!(resolve_font_family_in_db(db, family).is_some(), "{family}");
    }
}

#[test]
fn embedded_font_key_is_deterministic_and_installation_clears_old_aliases() {
    let mut fonts = bundled_fonts();
    assert_eq!(fonts.0.cache_key, bundled_fonts().0.cache_key);
    fonts.0.cache_key ^= 1;
    fonts
        .0
        .resolved_families
        .insert("test alias".into(), Some("Geom".into()));
    assert!(install_fonts(fonts));
    assert_eq!(
        super::super::resolve_font_family("test alias").as_deref(),
        Some("Geom")
    );
    assert!(install_fonts(bundled_fonts()));
    assert_eq!(super::super::resolve_font_family("test alias"), None);
    assert!(!install_fonts(bundled_fonts()));
}

#[test]
fn punctuation_only_family_never_matches_every_font() {
    let fonts = bundled_fonts();
    assert_eq!(
        resolve_font_family_in_db(fonts.0.font_system.db(), "---"),
        None
    );
}

#[test]
fn changing_the_font_set_invalidates_identical_text_rasters() {
    use super::super::{TextStyle, draw_text};
    use crate::{ClearColor, FrameSize, SoftwareBuffer};
    install_fonts(bundled_fonts());
    let draw = || {
        let mut buffer =
            SoftwareBuffer::solid(FrameSize::new(500, 100), ClearColor::opaque(0, 0, 0)).unwrap();
        draw_text(
            &mut buffer,
            0,
            0,
            "Hamburgefons 0123",
            TextStyle::new_px(ClearColor::opaque(255, 255, 255), 40),
        );
        buffer
    };
    let before = draw();
    let (mut db, hash) = bundled_database();
    db.set_sans_serif_family("Nunito");
    let changed = PreparedFonts(FontContext::from_database(db, hash.finish() ^ 1));
    assert!(install_fonts(changed));
    let after = draw();
    super::super::raster::clear_text_raster_cache();
    let fresh = draw();
    assert_ne!(before.pixels(), fresh.pixels());
    assert_eq!(after.pixels(), fresh.pixels());
}
