use std::rc::Rc;

use crate::{ClearColor, FrameSize, SoftwareBuffer};

use super::{
    TEXT_RASTER_CACHE, TEXT_RASTER_CACHE_LIMIT, TextBounds, TextRaster, TextRasterCache,
    TextRasterKey, TextStyle,
};
use crate::draw::text::fit_sensitive_single_line_text;

fn test_raster() -> Rc<TextRaster> {
    Rc::new(TextRaster {
        bounds: TextBounds {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        },
        width: 1,
        pixels: vec![255; 4],
    })
}

#[test]
fn cache_hit_reuses_the_same_pixel_allocation() {
    let color = ClearColor::opaque(255, 255, 255);
    let style = TextStyle::new(color, 2);
    let raster = test_raster();
    let mut cache = TextRasterCache {
        entries: Vec::new(),
    };
    cache.insert(
        TextRasterKey {
            text: "clock".into(),
            style: style.clone(),
            color,
        },
        Some(Rc::clone(&raster)),
    );

    let hit = cache.get("clock", &style, color).flatten();

    assert!(hit.is_some_and(|hit| Rc::ptr_eq(&hit, &raster)));
}

#[test]
fn cache_evicts_least_recently_used_entry() {
    let color = ClearColor::opaque(255, 255, 255);
    let style = TextStyle::new(color, 2);
    let mut cache = TextRasterCache {
        entries: Vec::new(),
    };
    for index in 0..TEXT_RASTER_CACHE_LIMIT {
        cache.insert(
            TextRasterKey {
                text: index.to_string(),
                style: style.clone(),
                color,
            },
            None,
        );
    }
    let _ = cache.get("0", &style, color);
    cache.insert(
        TextRasterKey {
            text: "new".into(),
            style: style.clone(),
            color,
        },
        None,
    );

    assert!(cache.get("1", &style, color).is_none());
    assert!(cache.get("0", &style, color).is_some());
}

#[test]
fn sensitive_text_bypasses_the_shared_raster_cache() {
    TEXT_RASTER_CACHE.with(|cache| cache.borrow_mut().entries.clear());
    let block = fit_sensitive_single_line_text(
        "hunter2",
        TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        128,
    );
    let mut buffer = SoftwareBuffer::new(FrameSize::new(128, 48)).expect("buffer");

    block.draw(&mut buffer, 0, 0);

    let contains_secret = TEXT_RASTER_CACHE.with(|cache| {
        cache
            .borrow()
            .entries
            .iter()
            .any(|(key, _)| key.text == "hunter2")
    });
    assert!(!contains_secret);
}
