//! P6 PPM reader/writer. grim `-t ppm` is the fallback capture format.

use crate::color::Rgb;
use std::io::{self, Read};

#[derive(Clone, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Frame {
    pub fn pixel(&self, x: u32, y: u32) -> Rgb {
        crate::kmeans::sample_pixel(self.width, self.height, &self.rgba, x, y)
    }

    #[allow(dead_code)]
    pub fn crop(&self, x: i32, y: i32, w: u32, h: u32) -> Frame {
        let w = w.max(1);
        let h = h.max(1);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for row in 0..h {
            for col in 0..w {
                let sx = (x + col as i32).clamp(0, self.width as i32 - 1) as u32;
                let sy = (y + row as i32).clamp(0, self.height as i32 - 1) as u32;
                let si = ((sy * self.width + sx) * 4) as usize;
                let di = ((row * w + col) * 4) as usize;
                rgba[di..di + 4].copy_from_slice(&self.rgba[si..si + 4]);
            }
        }
        Frame {
            width: w,
            height: h,
            rgba,
        }
    }

    pub fn scale_nearest(&self, factor: u32) -> Frame {
        let f = factor.max(1);
        let w = self.width * f;
        let h = self.height * f;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let sx = x / f;
                let sy = y / f;
                let si = ((sy * self.width + sx) * 4) as usize;
                let di = ((y * w + x) * 4) as usize;
                rgba[di..di + 4].copy_from_slice(&self.rgba[si..si + 4]);
            }
        }
        Frame {
            width: w,
            height: h,
            rgba,
        }
    }
}

fn skip_ws_and_comments(buf: &[u8], mut i: usize) -> usize {
    loop {
        while i < buf.len() && buf[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < buf.len() && buf[i] == b'#' {
            while i < buf.len() && buf[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        break;
    }
    i
}

fn read_token(buf: &[u8], i: usize) -> Result<(String, usize), io::Error> {
    let i = skip_ws_and_comments(buf, i);
    let start = i;
    let mut j = i;
    while j < buf.len() && !buf[j].is_ascii_whitespace() {
        j += 1;
    }
    if start == j {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "truncated ppm"));
    }
    Ok((String::from_utf8_lossy(&buf[start..j]).into_owned(), j))
}

pub fn parse_ppm(buf: &[u8]) -> io::Result<Frame> {
    if buf.len() < 3 || buf[0] != b'P' || (buf[1] != b'6' && buf[1] != b'3') {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not a ppm"));
    }
    let binary = buf[1] == b'6';
    let (w_s, i) = read_token(buf, 2)?;
    let (h_s, i) = read_token(buf, i)?;
    let (max_s, i) = read_token(buf, i)?;
    let width: u32 = w_s.parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad width"))?;
    let height: u32 = h_s.parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad height"))?;
    let max: u32 = max_s.parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad max"))?;
    if width == 0 || height == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "empty ppm"));
    }
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    if binary {
        let mut i = i;
        if i < buf.len() && (buf[i] == b'\n' || buf[i] == b' ' || buf[i] == b'\r') {
            i += 1;
        }
        let need = (width * height * 3) as usize;
        if buf.len().saturating_sub(i) < need {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "truncated p6"));
        }
        for p in 0..(width * height) as usize {
            let r = scale_sample(buf[i + p * 3], max);
            let g = scale_sample(buf[i + p * 3 + 1], max);
            let b = scale_sample(buf[i + p * 3 + 2], max);
            rgba[p * 4] = r;
            rgba[p * 4 + 1] = g;
            rgba[p * 4 + 2] = b;
            rgba[p * 4 + 3] = 255;
        }
    } else {
        let rest = String::from_utf8_lossy(&buf[i..]);
        let nums: Vec<u32> = rest
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        if nums.len() < (width * height * 3) as usize {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "truncated p3"));
        }
        for p in 0..(width * height) as usize {
            rgba[p * 4] = scale_sample_u32(nums[p * 3], max);
            rgba[p * 4 + 1] = scale_sample_u32(nums[p * 3 + 1], max);
            rgba[p * 4 + 2] = scale_sample_u32(nums[p * 3 + 2], max);
            rgba[p * 4 + 3] = 255;
        }
    }
    let _ = binary;
    Ok(Frame {
        width,
        height,
        rgba,
    })
}

fn scale_sample(v: u8, max: u32) -> u8 {
    if max == 255 {
        v
    } else {
        scale_sample_u32(v as u32, max)
    }
}

fn scale_sample_u32(v: u32, max: u32) -> u8 {
    if max == 0 {
        return 0;
    }
    ((v.min(max) * 255) / max) as u8
}

#[allow(dead_code)]
pub fn write_p3(frame: &Frame) -> Vec<u8> {
    let mut out = format!("P3\n{} {}\n255\n", frame.width, frame.height).into_bytes();
    for y in 0..frame.height {
        for x in 0..frame.width {
            let p = frame.pixel(x, y);
            out.extend_from_slice(format!("{} {} {} ", p.r, p.g, p.b).as_bytes());
        }
        out.push(b'\n');
    }
    out
}

pub fn load_path(path: &str) -> io::Result<Frame> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    if buf.starts_with(b"\x89PNG") {
        crate::pngbuf::read_png_bytes(&buf)
    } else {
        parse_ppm(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p3_round_trip() {
        let mut rgba = vec![0u8; 16];
        rgba[0] = 255;
        rgba[1] = 0;
        rgba[2] = 0;
        rgba[3] = 255;
        rgba[4] = 0;
        rgba[5] = 255;
        rgba[6] = 0;
        rgba[7] = 255;
        rgba[8] = 0;
        rgba[9] = 0;
        rgba[10] = 255;
        rgba[11] = 255;
        rgba[12] = 255;
        rgba[13] = 255;
        rgba[14] = 255;
        rgba[15] = 255;
        let frame = Frame {
            width: 2,
            height: 2,
            rgba,
        };
        let bytes = write_p3(&frame);
        let back = parse_ppm(&bytes).unwrap();
        assert_eq!(back.width, 2);
        assert_eq!(back.pixel(0, 0), Rgb::new(255, 0, 0));
        assert_eq!(back.pixel(1, 0), Rgb::new(0, 255, 0));
    }
}
