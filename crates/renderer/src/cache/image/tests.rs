use super::{cached_size, read_pixels, write_pixels};
use crate::FrameSize;
use std::fs;

#[test]
fn invalid_image_files_are_misses_and_can_be_repaired() {
    let root = std::env::temp_dir().join(format!("veila-cache-invalid-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("entry");
    let size = FrameSize::new(2, 3);
    let pixels = vec![59; 24];
    for magic in [b"KWYBG001", b"KWYIMG01", b"VEILAVA1"] {
        write_pixels(&path, magic, size, &pixels).unwrap();
        let valid = fs::read(&path).unwrap();
        let mut trailing = valid.clone();
        trailing.push(0);
        let mut bad_magic = valid.clone();
        bad_magic[0] = 0;
        let mut excessive = valid.clone();
        excessive[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut zero = valid.clone();
        zero[8..12].copy_from_slice(&0u32.to_le_bytes());
        for corrupt in [
            vec![],
            valid[..15].to_vec(),
            valid[..20].to_vec(),
            trailing,
            bad_magic,
            excessive,
            zero,
        ] {
            fs::write(&path, corrupt).unwrap();
            assert!(read_pixels(&path, magic, 16_384, None).is_none());
            assert!(cached_size(&path, magic, 16_384).is_none());
            assert!(
                path.exists(),
                "readers must not unlink a writer's replacement"
            );
            write_pixels(&path, magic, size, &pixels).unwrap();
            assert_eq!(
                read_pixels(&path, magic, 16_384, Some(size)),
                Some((size, pixels.clone()))
            );
        }
        assert!(read_pixels(&path, magic, 16_384, Some(FrameSize::new(3, 2))).is_none());
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_pixel_count_cannot_replace_a_valid_cache() {
    let root = std::env::temp_dir().join(format!("veila-cache-size-{}", std::process::id()));
    let path = root.join("entry");
    write_pixels(&path, b"KWYBG001", FrameSize::new(1, 1), &[0; 4]).unwrap();
    let original = fs::read(&path).unwrap();
    assert!(write_pixels(&path, b"KWYBG001", FrameSize::new(2, 1), &[0; 4]).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    fs::remove_dir_all(root).unwrap();
}
