use crate::ocr::have_bin;
use crate::ppm::{parse_ppm, Frame};
use std::process::{Command, Stdio};

pub fn available() -> bool {
    have_bin("grim")
}

fn run_grim(args: &[&str]) -> Result<Frame, String> {
    if !available() {
        return Err("grim not found".into());
    }
    let out = Command::new("grim")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    parse_ppm(&out.stdout).map_err(|e| e.to_string())
}

/// Cursor excluded: never pass grim `-c`.
/// `x,y,w,h` are output-local physical pixels.
pub fn capture_region_on(output: &str, x: i32, y: i32, w: i32, h: i32) -> Result<Frame, String> {
    let geom = format!("{},{} {}x{}", x, y, w.max(1), h.max(1));
    if !output.is_empty() {
        if let Ok(frame) = run_grim(&["-t", "ppm", "-o", output, "-g", &geom, "-"]) {
            return Ok(frame);
        }
    }
    run_grim(&["-t", "ppm", "-g", &geom, "-"])
}

#[allow(dead_code)]
pub fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<Frame, String> {
    capture_region_on("", x, y, w, h)
}

#[allow(dead_code)]
pub fn capture_monitor(name: &str) -> Result<Frame, String> {
    run_grim(&["-t", "ppm", "-o", name, "-"])
}

pub fn capture_full() -> Result<Frame, String> {
    run_grim(&["-t", "ppm", "-"])
}
