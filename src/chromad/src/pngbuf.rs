use crate::ppm::Frame;
use png::{BitDepth, ColorType, Decoder, Encoder};
use std::fs::File;
use std::io::{self, Cursor};
use std::path::Path;

pub fn write_png(path: &Path, frame: &Frame) -> io::Result<()> {
    let file = File::create(path)?;
    let mut enc = Encoder::new(file, frame.width, frame.height);
    enc.set_color(ColorType::Rgba);
    enc.set_depth(BitDepth::Eight);
    let mut writer = enc.write_header()?;
    writer
        .write_image_data(&frame.rgba)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

pub fn read_png_bytes(bytes: &[u8]) -> io::Result<Frame> {
    let cursor = Cursor::new(bytes);
    let decoder = Decoder::new(cursor);
    let mut reader = decoder
        .read_info()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let width = info.width;
    let height = info.height;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    match info.color_type {
        ColorType::Rgba => {
            rgba.copy_from_slice(&buf[..info.buffer_size()]);
        }
        ColorType::Rgb => {
            for i in 0..(width * height) as usize {
                rgba[i * 4] = buf[i * 3];
                rgba[i * 4 + 1] = buf[i * 3 + 1];
                rgba[i * 4 + 2] = buf[i * 3 + 2];
                rgba[i * 4 + 3] = 255;
            }
        }
        ColorType::Grayscale => {
            for i in 0..(width * height) as usize {
                let v = buf[i];
                rgba[i * 4] = v;
                rgba[i * 4 + 1] = v;
                rgba[i * 4 + 2] = v;
                rgba[i * 4 + 3] = 255;
            }
        }
        ColorType::GrayscaleAlpha => {
            for i in 0..(width * height) as usize {
                let v = buf[i * 2];
                rgba[i * 4] = v;
                rgba[i * 4 + 1] = v;
                rgba[i * 4 + 2] = v;
                rgba[i * 4 + 3] = buf[i * 2 + 1];
            }
        }
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported png color {other:?}"),
            ))
        }
    }
    Ok(Frame {
        width,
        height,
        rgba,
    })
}

#[allow(dead_code)]
pub fn read_png(path: &Path) -> io::Result<Frame> {
    let bytes = std::fs::read(path)?;
    read_png_bytes(&bytes)
}
