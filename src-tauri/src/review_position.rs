/// Place the review window near the capture cursor in physical coordinates.
/// Work-area origins may be negative; the taskbar is excluded from its extent.
pub fn near_cursor(
    cursor: (i32, i32),
    window: (u32, u32),
    origin: (i32, i32),
    area: (u32, u32),
) -> (i32, i32) {
    fn axis(cursor: i32, size: u32, start: i32, extent: u32) -> i32 {
        let low = i64::from(start);
        // An oversized window stays anchored at the work-area origin. Placement
        // alone cannot qualify layouts larger than the available display.
        let high = (low + i64::from(extent) - i64::from(size)).max(low);
        (i64::from(cursor) + 18)
            .clamp(low, high)
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }
    (
        axis(cursor.0, window.0, origin.0, area.0),
        axis(cursor.1, window.1, origin.1, area.1),
    )
}

#[cfg(test)]
mod tests {
    use super::near_cursor;
    #[test]
    fn observed_bottom_right_capture_keeps_accept_on_screen() {
        assert_eq!(
            near_cursor((2318, 1439), (436, 469), (0, 0), (2560, 1392)),
            (2124, 923)
        );
    }
    #[test]
    fn negative_monitor_origin_is_preserved() {
        assert_eq!(
            near_cursor((-1800, 100), (420, 460), (-1920, 0), (1920, 1032)),
            (-1782, 118)
        );
    }
    #[test]
    fn top_taskbar_and_left_edge_are_respected() {
        assert_eq!(
            near_cursor((-1930, 0), (420, 460), (-1920, 48), (1920, 984)),
            (-1912, 48)
        );
    }
    #[test]
    fn oversized_window_does_not_panic_or_drift_off_origin() {
        assert_eq!(
            near_cursor((100, 100), (900, 900), (-200, 50), (400, 300)),
            (-200, 50)
        );
    }
    #[test]
    fn arithmetic_does_not_wrap_at_coordinate_limits() {
        assert_eq!(
            near_cursor(
                (i32::MAX, i32::MIN),
                (10, 10),
                (i32::MAX - 100, i32::MIN),
                (100, 100)
            ),
            (i32::MAX - 10, i32::MIN + 18)
        );
    }
}
