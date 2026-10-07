use super::*;

const fn r(x: i32, y: i32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

const fn z(edge: Edge, px: u32) -> Zone {
    Zone { edge, px }
}

#[test]
fn zones_come_off_the_frame() {
    // name, frame, zones, work
    let cases: Vec<(&str, Rect, Vec<Zone>, Rect)> = vec![
        (
            "no panels",
            r(0, 0, 1920, 1080),
            vec![],
            r(0, 0, 1920, 1080),
        ),
        (
            "bar on top",
            r(0, 0, 1920, 1080),
            vec![z(Edge::Top, 32)],
            r(0, 32, 1920, 1048),
        ),
        (
            "bar and dock",
            r(0, 0, 1920, 1080),
            vec![z(Edge::Top, 32), z(Edge::Bottom, 68)],
            r(0, 32, 1920, 980),
        ),
        (
            "a dock that hides reserves nothing",
            r(0, 0, 1920, 1080),
            vec![z(Edge::Top, 32), z(Edge::Bottom, 0)],
            r(0, 32, 1920, 1048),
        ),
        (
            "second output keeps its origin",
            r(2560, -200, 2560, 1440),
            vec![z(Edge::Top, 32), z(Edge::Bottom, 68)],
            r(2560, -168, 2560, 1340),
        ),
        (
            "side dock",
            r(0, 0, 1920, 1080),
            vec![z(Edge::Left, 80), z(Edge::Right, 10)],
            r(80, 0, 1830, 1080),
        ),
        (
            "zones on one edge add",
            r(0, 0, 1000, 1000),
            vec![z(Edge::Top, 30), z(Edge::Top, 20)],
            r(0, 50, 1000, 950),
        ),
        (
            "more than the output is empty, never negative",
            r(0, 0, 100, 100),
            vec![z(Edge::Top, 80), z(Edge::Bottom, 80)],
            r(0, 80, 100, 0),
        ),
    ];
    for (name, frame, zones, want) in cases {
        assert_eq!(work_rect(frame, &zones), want, "{name}");
    }
}

#[test]
fn an_area_carries_the_frame_and_the_scale() {
    let area = work_area(7, "DP-1", r(0, 0, 2560, 1440), 180, &[z(Edge::Top, 32)]);
    assert_eq!(
        area,
        WorkArea {
            id: 7,
            name: "DP-1".to_owned(),
            frame: r(0, 0, 2560, 1440),
            work: r(0, 32, 2560, 1408),
            scale: 180,
        }
    );
}
