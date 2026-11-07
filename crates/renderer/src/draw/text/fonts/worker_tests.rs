use super::super::bundled_fonts;
use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::time::Duration;

#[test]
fn reloads_coalesce_while_loading_without_blocking_the_caller() {
    let (started, loading) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let (ready, results) = mpsc::channel();
    let count = AtomicUsize::new(0);
    let resume = Mutex::new(resume);
    let loader = FontLoader::with_prepare(
        move |generation, _| {
            ready.send(generation).unwrap();
        },
        move |requests| {
            if count.fetch_add(1, Ordering::SeqCst) == 0 {
                started.send(()).unwrap();
                resume.lock().unwrap().recv().unwrap();
            } else {
                assert_eq!(requests.len(), MAX_CONFIGURED_FONT_REQUESTS);
            }
            bundled_fonts()
        },
    )
    .unwrap();
    loader.request(1, vec![]);
    loading.recv_timeout(Duration::from_secs(2)).unwrap();
    loader.request(2, vec![]);
    loader.request(
        3,
        vec![
            FontRequest {
                family: "Geom".into(),
                weight: 600,
                italic: false
            };
            MAX_CONFIGURED_FONT_REQUESTS + 10
        ],
    );
    release.send(()).unwrap();
    assert_eq!(results.recv_timeout(Duration::from_secs(2)).unwrap(), 1);
    assert_eq!(results.recv_timeout(Duration::from_secs(2)).unwrap(), 3);
    assert!(results.try_recv().is_err());
}

#[test]
fn dropping_the_loader_never_joins_blocked_optional_work() {
    let (started, loading) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let resume = Mutex::new(resume);
    let loader = FontLoader::with_prepare(
        |_, _| {},
        move |_| {
            started.send(()).unwrap();
            resume.lock().unwrap().recv().unwrap();
            bundled_fonts()
        },
    )
    .unwrap();
    loader.request(1, vec![]);
    loading.recv_timeout(Duration::from_secs(2)).unwrap();
    let (done, dropped) = mpsc::channel();
    let dropping = thread::spawn(move || {
        drop(loader);
        done.send(()).unwrap();
    });
    let nonblocking = dropped.recv_timeout(Duration::from_millis(200)).is_ok();
    release.send(()).unwrap();
    dropping.join().unwrap();
    assert!(nonblocking);
}
