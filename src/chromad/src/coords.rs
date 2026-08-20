//! Logical (Hyprland) ↔ physical (capture buffer) mapping.
//! Isolated so 1x / 1.5x / 2x and dual-monitor straddles share one module.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Monitor {
    pub name: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default = "one")]
    pub scale: f64,
    #[serde(default)]
    pub focused: bool,
}

fn one() -> f64 {
    1.0
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn intersect(self, other: Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = (self.x + self.w).min(other.x + other.w);
        let b = (self.y + self.h).min(other.y + other.h);
        if r <= x || b <= y {
            Rect {
                x,
                y,
                w: 0,
                h: 0,
            }
        } else {
            Rect {
                x,
                y,
                w: r - x,
                h: b - y,
            }
        }
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0 || self.h <= 0
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Mapping {
    pub logical_x: f64,
    pub logical_y: f64,
    pub physical_x: i32,
    pub physical_y: i32,
    pub monitor: String,
    pub scale: f64,
}

pub fn logical_size(mon: &Monitor) -> (f64, f64, f64) {
    let scale = if mon.scale <= 0.0 { 1.0 } else { mon.scale };
    (mon.width as f64 / scale, mon.height as f64 / scale, scale)
}

pub fn containing<'a>(monitors: &'a [Monitor], x: f64, y: f64) -> Option<&'a Monitor> {
    let mut focused = None;
    for m in monitors {
        let (lw, lh, _) = logical_size(m);
        if x >= m.x as f64 && y >= m.y as f64 && x < m.x as f64 + lw && y < m.y as f64 + lh {
            return Some(m);
        }
        if m.focused {
            focused = Some(m);
        }
    }
    focused.or(monitors.first())
}

pub fn to_physical(mon: &Monitor, lx: f64, ly: f64) -> Mapping {
    let (_, _, scale) = logical_size(mon);
    Mapping {
        logical_x: lx,
        logical_y: ly,
        physical_x: ((lx - mon.x as f64) * scale).round() as i32,
        physical_y: ((ly - mon.y as f64) * scale).round() as i32,
        monitor: mon.name.clone(),
        scale,
    }
}

pub fn region_around(x: f64, y: f64, size: i32, monitors: &[Monitor]) -> Rect {
    let half = size / 2;
    let mon = containing(monitors, x, y);
    let mut rx = (x as i32) - half;
    let mut ry = (y as i32) - half;
    if let Some(m) = mon {
        let (lw, lh, _) = logical_size(m);
        let max_x = m.x + (lw as i32) - size;
        let max_y = m.y + (lh as i32) - size;
        rx = rx.clamp(m.x, max_x.max(m.x));
        ry = ry.clamp(m.y, max_y.max(m.y));
    }
    Rect {
        x: rx,
        y: ry,
        w: size,
        h: size,
    }
}

pub fn monitor_rect(mon: &Monitor) -> Rect {
    let (lw, lh, _) = logical_size(mon);
    Rect {
        x: mon.x,
        y: mon.y,
        w: lw.round() as i32,
        h: lh.round() as i32,
    }
}

pub fn clamp_window_to_monitor(win: Rect, mon: &Monitor) -> Rect {
    win.intersect(monitor_rect(mon))
}

/// Output-local physical rect for Wayland/grim. `logical` stays in Hyprland space for QML.
#[derive(Clone, Debug)]
pub struct MappedCapture {
    pub output: String,
    pub logical: Rect,
    pub physical: Rect,
    pub scale: f64,
}

pub fn map_capture(logical: Rect, monitors: &[Monitor]) -> Option<MappedCapture> {
    let cx = logical.x as f64 + (logical.w as f64) / 2.0;
    let cy = logical.y as f64 + (logical.h as f64) / 2.0;
    let mon = containing(monitors, cx, cy)?;
    Some(map_rect_on(logical, mon))
}

pub fn map_rect_on(logical: Rect, mon: &Monitor) -> MappedCapture {
    let (_, _, scale) = logical_size(mon);
    let physical = Rect {
        x: ((logical.x - mon.x) as f64 * scale).round() as i32,
        y: ((logical.y - mon.y) as f64 * scale).round() as i32,
        w: ((logical.w as f64) * scale).round().max(1.0) as i32,
        h: ((logical.h as f64) * scale).round().max(1.0) as i32,
    };
    MappedCapture {
        output: mon.name.clone(),
        logical,
        physical,
        scale,
    }
}

pub fn map_monitor(mon: &Monitor) -> MappedCapture {
    let (_, _, scale) = logical_size(mon);
    MappedCapture {
        output: mon.name.clone(),
        logical: monitor_rect(mon),
        physical: Rect {
            x: 0,
            y: 0,
            w: mon.width.max(1),
            h: mon.height.max(1),
        },
        scale,
    }
}

pub fn parse_monitors(json: &str) -> Result<Vec<Monitor>, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mons(path: &str) -> Vec<Monitor> {
        parse_monitors(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn fixture(name: &str) -> String {
        format!(
            "{}/../../tests/fixtures/{}",
            env!("CARGO_MANIFEST_DIR"),
            name
        )
    }

    #[test]
    fn scale_1x_physical_equals_logical() {
        let m = &mons(&fixture("monitors-1x.json"))[0];
        let map = to_physical(m, 100.0, 50.0);
        assert_eq!(map.physical_x, 100);
        assert_eq!(map.physical_y, 50);
        assert_eq!(map.scale, 1.0);
    }

    #[test]
    fn scale_1_5_multiplies() {
        let m = &mons(&fixture("monitors-1.5x.json"))[0];
        let (lw, lh, scale) = logical_size(m);
        assert!((lw - 1920.0).abs() < 0.01);
        assert!((lh - 1200.0).abs() < 0.01);
        let map = to_physical(m, 100.0, 50.0);
        assert_eq!(map.physical_x, 150);
        assert_eq!(map.physical_y, 75);
        assert_eq!(scale, 1.5);
    }

    #[test]
    fn scale_2x_multiplies() {
        let m = &mons(&fixture("monitors-2x.json"))[0];
        let map = to_physical(m, 10.0, 10.0);
        assert_eq!(map.physical_x, 20);
        assert_eq!(map.physical_y, 20);
    }

    #[test]
    fn straddling_window_clipped_to_left_monitor() {
        let m = &mons(&fixture("monitors-dual.json"))[0];
        let win = Rect {
            x: 400,
            y: 200,
            w: 2200,
            h: 800,
        };
        let clipped = clamp_window_to_monitor(win, m);
        assert_eq!(clipped.w, 1920 - 400);
        assert_eq!(clipped.h, 800);
        assert_eq!(clipped.x, 400);
    }

    #[test]
    fn region_clamps_to_origin() {
        let mons = mons(&fixture("monitors-1x.json"));
        let r = region_around(10.0, 10.0, 128, &mons);
        assert_eq!(r.x, 0);
        assert_eq!(r.y, 0);
        assert_eq!(r.w, 128);
    }

    #[test]
    fn map_capture_1_5x_is_output_local_physical() {
        let mons = mons(&fixture("monitors-1.5x.json"));
        let logical = Rect {
            x: 100,
            y: 50,
            w: 128,
            h: 128,
        };
        let mapped = map_capture(logical, &mons).unwrap();
        assert_eq!(mapped.output, "eDP-1");
        assert_eq!(mapped.logical, logical);
        assert_eq!(mapped.physical.x, 150);
        assert_eq!(mapped.physical.y, 75);
        assert_eq!(mapped.physical.w, 192);
        assert_eq!(mapped.physical.h, 192);
        assert_eq!(mapped.scale, 1.5);
    }

    #[test]
    fn map_monitor_physical_is_buffer_size() {
        let m = &mons(&fixture("monitors-2x.json"))[0];
        let mapped = map_monitor(m);
        assert_eq!(mapped.physical.x, 0);
        assert_eq!(mapped.physical.y, 0);
        assert_eq!(mapped.physical.w, 2560);
        assert_eq!(mapped.physical.h, 1600);
        assert_eq!(mapped.logical.w, 1280);
        assert_eq!(mapped.logical.h, 800);
    }
}
