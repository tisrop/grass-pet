#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn snap_rect(bounds: Rect, work_area: Rect, threshold: i32) -> Rect {
    let mut snapped = bounds;
    let threshold = i64::from(threshold.max(0));
    let near = |left: i64, right: i64| (left - right).abs() < threshold;

    if near(i64::from(bounds.x), i64::from(work_area.x)) {
        snapped.x = work_area.x;
    }
    if near(
        i64::from(bounds.x) + i64::from(bounds.width),
        i64::from(work_area.x) + i64::from(work_area.width),
    ) {
        snapped.x = work_area.x + work_area.width as i32 - bounds.width as i32;
    }
    if near(i64::from(bounds.y), i64::from(work_area.y)) {
        snapped.y = work_area.y;
    }
    if near(
        i64::from(bounds.y) + i64::from(bounds.height),
        i64::from(work_area.y) + i64::from(work_area.height),
    ) {
        snapped.y = work_area.y + work_area.height as i32 - bounds.height as i32;
    }

    snapped
}

#[cfg(test)]
mod tests {
    use super::{snap_rect, Rect};

    const AREA: Rect = Rect {
        x: 0,
        y: 25,
        width: 1440,
        height: 875,
    };

    #[test]
    fn snaps_each_near_edge_without_changing_size() {
        assert_eq!(
            snap_rect(
                Rect {
                    x: 12,
                    y: 300,
                    width: 192,
                    height: 192
                },
                AREA,
                20
            )
            .x,
            0
        );
        assert_eq!(
            snap_rect(
                Rect {
                    x: 1238,
                    y: 300,
                    width: 192,
                    height: 192
                },
                AREA,
                20
            )
            .x,
            1248
        );
        assert_eq!(
            snap_rect(
                Rect {
                    x: 400,
                    y: 34,
                    width: 192,
                    height: 192
                },
                AREA,
                20
            )
            .y,
            25
        );
        assert_eq!(
            snap_rect(
                Rect {
                    x: 400,
                    y: 698,
                    width: 192,
                    height: 192
                },
                AREA,
                20
            )
            .y,
            708
        );
    }

    #[test]
    fn leaves_windows_outside_the_threshold_unchanged() {
        let bounds = Rect {
            x: 200,
            y: 300,
            width: 192,
            height: 192,
        };
        assert_eq!(snap_rect(bounds, AREA, 20), bounds);
    }

    #[test]
    fn supports_work_areas_with_negative_origins() {
        let area = Rect {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let bounds = Rect {
            x: -1911,
            y: 400,
            width: 240,
            height: 240,
        };
        assert_eq!(snap_rect(bounds, area, 20).x, -1920);
    }
}
