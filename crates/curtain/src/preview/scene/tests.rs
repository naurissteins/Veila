use super::*;

#[test]
fn preview_clock_datetime_reuses_local_date_with_requested_time() {
    let datetime = preview_clock_datetime(PreviewClockTime {
        hour: 21,
        minute: 54,
    });

    assert_eq!(datetime.hour(), 21);
    assert_eq!(datetime.minute(), 54);
    assert_eq!(datetime.second(), 0);
}
