//! sRGB color science shared with the QML HUD (HEX / RGB / HSL / OKLCH / WCAG).

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(input: &str) -> Option<Self> {
        let s = input.trim().trim_start_matches('#');
        let s = if s.len() == 3 {
            format!(
                "{}{}{}{}{}{}",
                s.as_bytes()[0] as char,
                s.as_bytes()[0] as char,
                s.as_bytes()[1] as char,
                s.as_bytes()[1] as char,
                s.as_bytes()[2] as char,
                s.as_bytes()[2] as char
            )
        } else if s.len() == 8 {
            s[..6].to_string()
        } else {
            s.to_string()
        };
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }

    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn rgb_string(self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    pub fn hsl(self) -> (f64, f64, f64) {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        let d = max - min;
        if d == 0.0 {
            return (0.0, 0.0, l * 100.0);
        }
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let h = if max == r {
            ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
        } else if max == g {
            ((b - r) / d + 2.0) / 6.0
        } else {
            ((r - g) / d + 4.0) / 6.0
        };
        (h * 360.0, s * 100.0, l * 100.0)
    }

    pub fn hsl_string(self) -> String {
        let (h, s, l) = self.hsl();
        format!("hsl({:.1}, {:.1}%, {:.1}%)", h, s, l)
    }

    pub fn oklch(self) -> Oklch {
        let lab = rgb_to_oklab(self);
        let c = (lab.1 * lab.1 + lab.2 * lab.2).sqrt();
        let mut h = lab.2.atan2(lab.1).to_degrees();
        if h < 0.0 {
            h += 360.0;
        }
        Oklch { l: lab.0, c, h }
    }

    pub fn oklch_string(self) -> String {
        let lch = self.oklch();
        format!("oklch({:.3} {:.3} {:.1})", lch.l, lch.c, lch.h)
    }

    pub fn relative_luminance(self) -> f64 {
        0.2126 * srgb_to_linear(self.r)
            + 0.7152 * srgb_to_linear(self.g)
            + 0.0722 * srgb_to_linear(self.b)
    }
}

#[allow(dead_code)]
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let l1 = a.relative_luminance();
    let l2 = b.relative_luminance();
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

pub fn srgb_to_linear(c: u8) -> f64 {
    let x = c as f64 / 255.0;
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

fn rgb_to_oklab(rgb: Rgb) -> (f64, f64, f64) {
    let lr = srgb_to_linear(rgb.r);
    let lg = srgb_to_linear(rgb.g);
    let lb = srgb_to_linear(rgb.b);
    let l = 0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb;
    let m = 0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb;
    let s = 0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb;
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();
    (
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    )
}

pub fn pixel_json(rgb: Rgb) -> serde_json::Value {
    serde_json::json!({
        "hex": rgb.hex(),
        "r": rgb.r,
        "g": rgb.g,
        "b": rgb.b,
        "rgb": rgb.rgb_string(),
        "hsl": rgb.hsl_string(),
        "oklch": rgb.oklch_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let c = Rgb::from_hex("#ff00aa").unwrap();
        assert_eq!(c, Rgb::new(255, 0, 170));
        assert_eq!(c.hex(), "#ff00aa");
        assert_eq!(Rgb::from_hex("ABC").unwrap().hex(), "#aabbcc");
    }

    #[test]
    fn wcag_black_white() {
        let ratio = contrast_ratio(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));
        assert!((ratio - 21.0).abs() < 0.05);
    }

    #[test]
    fn red_oklch_is_chromatic() {
        let lch = Rgb::new(255, 0, 0).oklch();
        assert!(lch.c > 0.2);
        assert!(lch.l > 0.5 && lch.l < 0.7);
    }
}
