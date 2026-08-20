//! Seeded k-means on a downscaled RGBA frame. k=6 is the palette size.

use crate::color::Rgb;

const SEED: u64 = 42;

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9e3779b97f4a7c15 } else { seed },
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        (z ^ (z >> 31)) as u32
    }

    fn gen_range(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u32() as usize) % n
    }
}

#[derive(Clone, Copy)]
struct Sample {
    r: f64,
    g: f64,
    b: f64,
}

pub fn downscale_rgba(width: u32, height: u32, rgba: &[u8], max_side: u32) -> (u32, u32, Vec<u8>) {
    let max_side = max_side.max(1);
    let scale = (width.max(height) as f64) / max_side as f64;
    if scale <= 1.0 {
        return (width, height, rgba.to_vec());
    }
    let nw = ((width as f64) / scale).max(1.0).round() as u32;
    let nh = ((height as f64) / scale).max(1.0).round() as u32;
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        for x in 0..nw {
            let sx = ((x as f64 + 0.5) * scale).floor() as u32;
            let sy = ((y as f64 + 0.5) * scale).floor() as u32;
            let sx = sx.min(width - 1);
            let sy = sy.min(height - 1);
            let si = ((sy * width + sx) * 4) as usize;
            let di = ((y * nw + x) * 4) as usize;
            out[di..di + 4].copy_from_slice(&rgba[si..si + 4]);
        }
    }
    (nw, nh, out)
}

pub fn kmeans_palette(width: u32, height: u32, rgba: &[u8], k: usize) -> Vec<Rgb> {
    let k = k.max(1).min(12);
    let (_, _, small) = downscale_rgba(width, height, rgba, 64);
    let mut samples: Vec<Sample> = Vec::new();
    for chunk in small.chunks(4) {
        if chunk.len() < 4 {
            break;
        }
        if chunk[3] < 16 {
            continue;
        }
        samples.push(Sample {
            r: chunk[0] as f64,
            g: chunk[1] as f64,
            b: chunk[2] as f64,
        });
    }
    if samples.is_empty() {
        return vec![Rgb::new(0, 0, 0); k];
    }
    let mut rng = Rng::new(SEED);
    let mut centers: Vec<Sample> = Vec::with_capacity(k);
    let first = samples[rng.gen_range(samples.len())];
    centers.push(first);
    while centers.len() < k {
        let mut best_i = 0;
        let mut best_d = -1.0;
        for (i, s) in samples.iter().enumerate() {
            let mut d = f64::MAX;
            for c in &centers {
                let dist = (s.r - c.r).powi(2) + (s.g - c.g).powi(2) + (s.b - c.b).powi(2);
                if dist < d {
                    d = dist;
                }
            }
            if d > best_d {
                best_d = d;
                best_i = i;
            }
        }
        centers.push(samples[best_i]);
        if best_d <= 1.0 {
            break;
        }
    }
    let k_eff = centers.len();
    let mut assign = vec![0usize; samples.len()];
    for _ in 0..24 {
        let mut changed = false;
        for (i, s) in samples.iter().enumerate() {
            let mut best = 0;
            let mut best_d = f64::MAX;
            for (ci, c) in centers.iter().enumerate() {
                let d = (s.r - c.r).powi(2) + (s.g - c.g).powi(2) + (s.b - c.b).powi(2);
                if d < best_d {
                    best_d = d;
                    best = ci;
                }
            }
            if assign[i] != best {
                assign[i] = best;
                changed = true;
            }
        }
        let mut acc = vec![(0.0, 0.0, 0.0, 0.0); k_eff];
        for (i, s) in samples.iter().enumerate() {
            let a = &mut acc[assign[i]];
            a.0 += s.r;
            a.1 += s.g;
            a.2 += s.b;
            a.3 += 1.0;
        }
        for (i, c) in centers.iter_mut().enumerate() {
            if acc[i].3 > 0.0 {
                c.r = acc[i].0 / acc[i].3;
                c.g = acc[i].1 / acc[i].3;
                c.b = acc[i].2 / acc[i].3;
            }
        }
        if !changed {
            break;
        }
    }
    let mut colors: Vec<Rgb> = centers
        .into_iter()
        .map(|c| {
            Rgb::new(
                c.r.round().clamp(0.0, 255.0) as u8,
                c.g.round().clamp(0.0, 255.0) as u8,
                c.b.round().clamp(0.0, 255.0) as u8,
            )
        })
        .collect();
    colors.sort_by(|a, b| {
        a.relative_luminance()
            .partial_cmp(&b.relative_luminance())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    colors
}

pub fn sample_pixel(width: u32, height: u32, rgba: &[u8], x: u32, y: u32) -> Rgb {
    let x = x.min(width.saturating_sub(1));
    let y = y.min(height.saturating_sub(1));
    let i = ((y * width + x) * 4) as usize;
    if i + 2 >= rgba.len() {
        return Rgb::new(0, 0, 0);
    }
    Rgb::new(rgba[i], rgba[i + 1], rgba[i + 2])
}

pub fn nearest_hexes(colors: &[Rgb]) -> Vec<String> {
    colors.iter().map(|c| c.hex()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_split() -> (u32, u32, Vec<u8>) {
        let w = 32u32;
        let h = 16u32;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                if x < w / 2 {
                    rgba[i] = 220;
                    rgba[i + 1] = 20;
                    rgba[i + 2] = 20;
                } else {
                    rgba[i] = 20;
                    rgba[i + 1] = 40;
                    rgba[i + 2] = 220;
                }
                rgba[i + 3] = 255;
            }
        }
        (w, h, rgba)
    }

    #[test]
    fn seeded_kmeans_is_deterministic() {
        let (w, h, rgba) = solid_split();
        let a = kmeans_palette(w, h, &rgba, 2);
        let b = kmeans_palette(w, h, &rgba, 2);
        assert_eq!(a, b);
        assert_eq!(a.len(), 2);
        let hexes = nearest_hexes(&a);
        assert!(hexes.iter().any(|h| h.starts_with("#dc") || h.starts_with("#d4") || h.starts_with("#c8") || Rgb::from_hex(h).unwrap().r > 180));
        assert!(hexes.iter().any(|h| Rgb::from_hex(h).unwrap().b > 180));
    }

    #[test]
    fn center_pixel_of_red_half() {
        let (w, h, rgba) = solid_split();
        let p = sample_pixel(w, h, &rgba, 4, 8);
        assert!(p.r > 200 && p.b < 40);
    }
}
