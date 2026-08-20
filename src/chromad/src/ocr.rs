use crate::pngbuf;
use crate::ppm::Frame;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn have_bin(name: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path).any(|p| {
        let cand = p.join(name);
        cand.is_file()
    })
}

fn write_temp_png(frame: &Frame) -> std::io::Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!("chroma-ocr-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("shot.png");
    pngbuf::write_png(&path, frame)?;
    Ok(path)
}

pub fn run_tesseract(frame: &Frame) -> Result<String, String> {
    if !have_bin("tesseract") {
        return Err("tesseract not installed (pacman -S tesseract)".into());
    }
    let scaled = frame.scale_nearest(2);
    let path = write_temp_png(&scaled).map_err(|e| e.to_string())?;
    let out = Command::new("tesseract")
        .args([path.to_str().unwrap_or(""), "stdout", "-l", "eng"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&path);
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn run_zbar(frame: &Frame) -> Result<String, String> {
    if !have_bin("zbarimg") {
        return Err("zbarimg not installed (pacman -S zbar)".into());
    }
    let path = write_temp_png(frame).map_err(|e| e.to_string())?;
    let out = Command::new("zbarimg")
        .args(["-q", "--raw", path.to_str().unwrap_or("")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&path);
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if err.is_empty() {
            return Err("no QR/barcode found".into());
        }
        return Err(err);
    }
    Ok(text)
}

pub fn which_report() -> (bool, bool, bool) {
    (have_bin("grim"), have_bin("tesseract"), have_bin("zbarimg"))
}

pub fn wl_copy(text: &str) -> Result<(), String> {
    if !have_bin("wl-copy") {
        return Err("wl-copy missing".into());
    }
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("wl-copy failed".into())
    }
}
