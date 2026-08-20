use crate::color::Rgb;
use crate::ppm::Frame;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GridLayout {
    pub cell: u32,
    pub cols: u32,
    pub rows: u32,
    pub colors: Vec<Vec<String>>,
}

#[derive(Debug)]
pub struct CellResult {
    pub row: usize,
    pub col: usize,
    pub expected: String,
    pub got: String,
    pub ok: bool,
}

pub fn sample_grid(frame: &Frame, layout: &GridLayout) -> Vec<CellResult> {
    let mut out = Vec::new();
    let cell = layout.cell.max(1);
    for r in 0..layout.rows as usize {
        for c in 0..layout.cols as usize {
            let expected = layout
                .colors
                .get(r)
                .and_then(|row| row.get(c))
                .cloned()
                .unwrap_or_else(|| "#000000".into());
            let cx = (c as u32 * cell) + cell / 2;
            let cy = (r as u32 * cell) + cell / 2;
            let got = frame.pixel(cx, cy).hex();
            let exp = Rgb::from_hex(&expected).map(|x| x.hex()).unwrap_or(expected.clone());
            let ok = hex_close(&got, &exp, 12);
            out.push(CellResult {
                row: r,
                col: c,
                expected: exp,
                got,
                ok,
            })
        }
    }
    out
}

pub fn hex_close(a: &str, b: &str, tol: i32) -> bool {
    let pa = Rgb::from_hex(a);
    let pb = Rgb::from_hex(b);
    match (pa, pb) {
        (Some(x), Some(y)) => {
            (x.r as i32 - y.r as i32).abs() <= tol
                && (x.g as i32 - y.g as i32).abs() <= tol
                && (x.b as i32 - y.b as i32).abs() <= tol
        }
        _ => false,
    }
}

#[allow(dead_code)]
pub fn make_grid(layout: &GridLayout) -> Frame {
    let cell = layout.cell.max(1);
    let width = layout.cols * cell;
    let height = layout.rows * cell;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for r in 0..layout.rows {
        for c in 0..layout.cols {
            let hex = layout
                .colors
                .get(r as usize)
                .and_then(|row| row.get(c as usize))
                .map(|s| s.as_str())
                .unwrap_or("#000000");
            let rgb = Rgb::from_hex(hex).unwrap_or(Rgb::new(0, 0, 0));
            for y in (r * cell)..((r + 1) * cell) {
                for x in (c * cell)..((c + 1) * cell) {
                    let i = ((y * width + x) * 4) as usize;
                    rgba[i] = rgb.r;
                    rgba[i + 1] = rgb.g;
                    rgba[i + 2] = rgb.b;
                    rgba[i + 3] = 255;
                }
            }
        }
    }
    Frame {
        width,
        height,
        rgba,
    }
}

pub fn recursion_hit(frame: &Frame, sentinel: Rgb, stride: u32) -> bool {
    let stride = stride.max(1);
    let mut hits = 0u32;
    let mut n = 0u32;
    let mut y = 0;
    while y < frame.height {
        let mut x = 0;
        while x < frame.width {
            let p = frame.pixel(x, y);
            n += 1;
            if hex_close(&p.hex(), &sentinel.hex(), 10) {
                hits += 1;
            }
            x += stride;
        }
        y += stride;
    }
    n > 0 && (hits as f64) > (n as f64) * 0.08
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_samples_match_layout() {
        let layout: GridLayout = serde_json::from_str(
            "{\"cell\":8,\"cols\":2,\"rows\":2,\"colors\":[[\"#ff0000\",\"#00ff00\"],[\"#0000ff\",\"#ffffff\"]]}",
        )
        .unwrap();
        let frame = make_grid(&layout);
        let results = sample_grid(&frame, &layout);
        assert!(results.iter().all(|r| r.ok), "{results:?}");
    }

    #[test]
    fn sentinel_detects_chrome() {
        let mut frame = make_grid(&GridLayout {
            cell: 4,
            cols: 2,
            rows: 1,
            colors: vec![vec!["#112233".into(), "#ff2bd6".into()]],
        });
        // flood most of it with the sentinel so the 8% threshold trips
        for px in frame.rgba.chunks_mut(4) {
            px[0] = 0xff;
            px[1] = 0x2b;
            px[2] = 0xd6;
            px[3] = 255;
        }
        assert!(recursion_hit(&frame, Rgb::from_hex("#ff2bd6").unwrap(), 1));
    }
}
