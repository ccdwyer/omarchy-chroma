mod calibrate;
mod capture;
mod color;
mod coords;
mod grim;
mod kmeans;
mod ocr;
mod pngbuf;
mod ppm;
#[cfg(target_os = "linux")]
mod wayland;

use capture::{window_rect_on_focused, Capturer};
use color::{pixel_json, Rgb};
use coords::{region_around, Monitor, Rect};
use kmeans::{kmeans_palette, nearest_hexes};
use ppm::Frame;
use serde::Deserialize;
use serde_json::json;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const REGION: i32 = 128;
const STREAM_HZ: u64 = 30;
const SEED_NOTE: u64 = 42;

#[derive(Debug, Deserialize)]
struct Cmd {
    cmd: String,
    #[serde(default)]
    x: Option<f64>,
    #[serde(default)]
    y: Option<f64>,
    #[serde(default)]
    w: Option<i32>,
    #[serde(default)]
    h: Option<i32>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    factor: Option<u32>,
    #[serde(default)]
    dir: Option<String>,
    #[serde(default)]
    files: Option<serde_json::Value>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    sentinel: Option<String>,
}

fn emit(v: serde_json::Value) {
    let mut out = io::stdout();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

fn fail(msg: &str) {
    emit(json!({"ok": false, "event": "error", "error": msg}));
}

fn shm_dir() -> PathBuf {
    if let Ok(p) = std::env::var("CHROMA_SHM_DIR") {
        return PathBuf::from(p);
    }
    let pid = std::process::id();
    let base = if Path::new("/dev/shm").is_dir() {
        PathBuf::from("/dev/shm")
    } else if let Ok(x) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(x)
    } else {
        std::env::temp_dir()
    };
    base.join(format!("chroma-{pid}"))
}

static SHM_PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
static STOP: AtomicBool = AtomicBool::new(false);

fn remember_shm(dir: PathBuf) {
    let _ = SHM_PATH.set(dir);
    install_stop_signals();
}

fn remove_shm() {
    if let Some(p) = SHM_PATH.get() {
        let _ = std::fs::remove_dir_all(p);
    }
}

fn stop_requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

fn request_stop() {
    STOP.store(true, Ordering::SeqCst);
}

// Signal handler only flips a flag. Filesystem cleanup runs on the serve loop.
#[cfg(unix)]
fn install_stop_signals() {
    unsafe {
        libc::signal(libc::SIGINT, stop_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, stop_signal as *const () as libc::sighandler_t);
    }
}

#[cfg(not(unix))]
fn install_stop_signals() {}

#[cfg(unix)]
extern "C" fn stop_signal(_: i32) {
    request_stop();
}

struct Server {
    capturer: Capturer,
    monitors: Vec<Monitor>,
    cursor: (f64, f64),
    streaming: bool,
    frozen: Option<Frame>,
    freeze_rect: Option<Rect>,
    slot: u8,
    gen: u64,
    dir: PathBuf,
    last_frame_rect: Option<Rect>,
    last_frame_scale: f64,
}

impl Server {
    fn new() -> Self {
        let dir = shm_dir();
        let _ = std::fs::create_dir_all(&dir);
        remember_shm(dir.clone());
        let capturer = Capturer::negotiate();
        let monitors = capture::load_monitors();
        let cursor = capture::load_cursor().unwrap_or((0.0, 0.0));
        Self {
            capturer,
            monitors,
            cursor,
            streaming: false,
            frozen: None,
            freeze_rect: None,
            slot: 0,
            gen: 0,
            dir,
            last_frame_rect: None,
            last_frame_scale: 1.0,
        }
    }

    fn hello(&self) {
        let (grim, ocr, qr) = ocr::which_report();
        let live = self.capturer.live_backend.live();
        emit(json!({
            "ok": true,
            "event": "hello",
            "backend": self.capturer.live_backend.as_str(),
            "oneshotBackend": self.capturer.oneshot_backend.as_str(),
            "globals": self.capturer.globals,
            "ocr": ocr,
            "qr": qr,
            "grim": grim,
            "live": live,
            "pickMode": self.capturer.live_backend.pick_mode(),
            "region": REGION,
            "hz": STREAM_HZ,
            "kmeansSeed": SEED_NOTE,
            "shm": self.dir.to_string_lossy(),
        }));
    }

    fn write_slot(&mut self, frame: &Frame) -> Result<(PathBuf, u64, u8), String> {
        self.slot = if self.slot == 0 { 1 } else { 0 };
        self.gen += 1;
        let path = self.dir.join(format!("frame{}.png", self.slot));
        pngbuf::write_png(&path, frame).map_err(|e| e.to_string())?;
        Ok((path, self.gen, self.slot))
    }

    fn current_region(&self) -> Rect {
        region_around(self.cursor.0, self.cursor.1, REGION, &self.monitors)
    }

    fn capture_mapped(&mut self, mapped: &coords::MappedCapture, live: bool) -> Result<Frame, String> {
        self.capturer.capture_on(mapped, live)
    }

    fn capture_live(&mut self) -> Result<Frame, String> {
        let logical = self.current_region();
        let mapped = coords::map_capture(logical, &self.monitors)
            .ok_or_else(|| "no monitor for cursor".to_string())?;
        let frame = self.capture_mapped(&mapped, true)?;
        self.last_frame_rect = Some(mapped.logical);
        self.last_frame_scale = mapped.scale;
        Ok(frame)
    }

    fn emit_frame(&mut self, frame: &Frame, rect: Rect, scale: f64) -> Result<(), String> {
        let (path, n, slot) = self.write_slot(frame)?;
        let cx = (frame.width / 2).min(frame.width.saturating_sub(1));
        let cy = (frame.height / 2).min(frame.height.saturating_sub(1));
        let pixel = frame.pixel(cx, cy);
        emit(json!({
            "ok": true,
            "event": "frame",
            "path": path.to_string_lossy(),
            "n": n,
            "slot": slot,
            "x": rect.x,
            "y": rect.y,
            "w": rect.w,
            "h": rect.h,
            "scale": scale,
            "pixel": pixel_json(pixel)
        }));
        Ok(())
    }

    fn oneshot_rect(&mut self, kind: &str) -> Result<(Frame, Rect, f64), String> {
        if let Some(frozen) = self.frozen.clone() {
            let rect = self.freeze_rect.unwrap_or(Rect {
                x: 0,
                y: 0,
                w: frozen.width as i32,
                h: frozen.height as i32,
            });
            return Ok((frozen, rect, self.last_frame_scale));
        }
        let focused = self
            .monitors
            .iter()
            .find(|m| m.focused)
            .cloned()
            .or_else(|| self.monitors.first().cloned());
        match kind {
            "window" => {
                let logical = window_rect_on_focused(&self.monitors)
                    .ok_or_else(|| "no active window".to_string())?;
                let mapped = coords::map_capture(logical, &self.monitors)
                    .ok_or_else(|| "no monitor for window".to_string())?;
                let frame = self.capture_mapped(&mapped, false)?;
                Ok((frame, mapped.logical, mapped.scale))
            }
            "monitor" => {
                let m = focused.ok_or_else(|| "no monitor".to_string())?;
                let mapped = coords::map_monitor(&m);
                let frame = self.capture_mapped(&mapped, false)?;
                Ok((frame, mapped.logical, mapped.scale))
            }
            "region" => {
                let logical = self.current_region();
                let mapped = coords::map_capture(logical, &self.monitors)
                    .ok_or_else(|| "no monitor for cursor".to_string())?;
                let frame = self.capture_mapped(&mapped, false)?;
                Ok((frame, mapped.logical, mapped.scale))
            }
            _ => {
                if let Some(m) = focused {
                    let mapped = coords::map_monitor(&m);
                    let frame = self.capture_mapped(&mapped, false)?;
                    Ok((frame, mapped.logical, mapped.scale))
                } else {
                    let frame = self.capturer.capture_full()?;
                    let rect = Rect {
                        x: 0,
                        y: 0,
                        w: frame.width as i32,
                        h: frame.height as i32,
                    };
                    Ok((frame, rect, 1.0))
                }
            }
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd.cmd.as_str() {
            "hello" | "capabilities" => self.hello(),
            "cursor" => {
                if let (Some(x), Some(y)) = (cmd.x, cmd.y) {
                    self.cursor = (x, y);
                } else if let Some(c) = capture::load_cursor() {
                    self.cursor = c;
                }
                emit(json!({"ok": true, "event": "cursor", "x": self.cursor.0, "y": self.cursor.1}));
            }
            "monitors" => {
                self.monitors = capture::load_monitors();
                emit(json!({"ok": true, "event": "monitors", "count": self.monitors.len()}));
            }
            "start_stream" => {
                self.streaming = self.capturer.live_backend.live();
                emit(json!({
                    "ok": true,
                    "event": "stream",
                    "running": self.streaming,
                    "pickMode": self.capturer.live_backend.pick_mode()
                }));
            }
            "stop_stream" => {
                self.streaming = false;
                emit(json!({"ok": true, "event": "stream", "running": false}));
            }
            "pick" => match self.oneshot_rect("region") {
                Ok((frame, _rect, _scale)) => {
                    let p = frame.pixel(frame.width / 2, frame.height / 2);
                    let mut v = pixel_json(p);
                    v["ok"] = json!(true);
                    v["event"] = json!("pick");
                    v["pixel"] = pixel_json(p);
                    emit(v);
                }
                Err(e) => fail(&e),
            },
            "palette" => {
                let source = cmd.source.unwrap_or_else(|| "window".into());
                match self.oneshot_rect(&source) {
                    Ok((frame, rect, scale)) => {
                        let colors = kmeans_palette(frame.width, frame.height, &frame.rgba, 6);
                        emit(json!({
                            "ok": true,
                            "event": "palette",
                            "source": source,
                            "x": rect.x, "y": rect.y, "w": rect.w, "h": rect.h,
                            "scale": scale,
                            "colors": nearest_hexes(&colors)
                        }));
                    }
                    Err(e) => fail(&e),
                }
            }
            "oneshot" => {
                let kind = cmd.kind.unwrap_or_else(|| "monitor".into());
                match self.oneshot_rect(&kind) {
                    Ok((frame, rect, scale)) => {
                        if let Err(e) = self.emit_frame(&frame, rect, scale) {
                            fail(&e);
                        }
                    }
                    Err(e) => fail(&e),
                }
            }
            "freeze" => match self.oneshot_rect("monitor") {
                Ok((frame, rect, scale)) => {
                    self.frozen = Some(frame.clone());
                    self.freeze_rect = Some(rect);
                    self.last_frame_scale = scale;
                    if let Err(e) = self.emit_frame(&frame, rect, scale) {
                        fail(&e);
                    } else {
                        emit(json!({"ok": true, "event": "frozen", "factor": cmd.factor.unwrap_or(8)}));
                    }
                }
                Err(e) => fail(&e),
            },
            "unfreeze" => {
                self.frozen = None;
                self.freeze_rect = None;
                emit(json!({"ok": true, "event": "unfrozen"}));
            }
            "ocr" => match self.oneshot_rect(cmd.source.as_deref().unwrap_or("monitor")) {
                Ok((frame, _, _)) => match ocr::run_tesseract(&frame) {
                    Ok(text) => emit(json!({"ok": true, "event": "ocr", "text": text})),
                    Err(e) => fail(&e),
                },
                Err(e) => fail(&e),
            },
            "qr" => match self.oneshot_rect(cmd.source.as_deref().unwrap_or("monitor")) {
                Ok((frame, _, _)) => match ocr::run_zbar(&frame) {
                    Ok(text) => emit(json!({"ok": true, "event": "qr", "text": text})),
                    Err(e) => fail(&e),
                },
                Err(e) => fail(&e),
            },
            "write_theme" => {
                let dir = match cmd.dir {
                    Some(d) => PathBuf::from(d),
                    None => {
                        fail("write_theme needs dir");
                        return;
                    }
                };
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    fail(&e.to_string());
                    return;
                }
                if let Some(serde_json::Value::Object(map)) = cmd.files {
                    for (name, body) in map {
                        if name.contains("..") || name.contains('/') || name.contains('\\') {
                            fail("refusing path in theme file name");
                            return;
                        }
                        let text = body.as_str().unwrap_or("");
                        if let Err(e) = std::fs::write(dir.join(name), text) {
                            fail(&e.to_string());
                            return;
                        }
                    }
                    emit(json!({"ok": true, "event": "theme_written", "dir": dir.to_string_lossy()}));
                } else {
                    fail("write_theme needs files");
                }
            }
            "validate_theme" => {
                let dir = match cmd.dir {
                    Some(d) => PathBuf::from(d),
                    None => {
                        fail("validate_theme needs dir");
                        return;
                    }
                };
                let path = dir.join("colors.toml");
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        let ok = text.contains("background") && text.contains("accent") && text.contains("foreground");
                        emit(json!({"ok": ok, "event": "theme_valid", "path": path.to_string_lossy()}));
                    }
                    Err(e) => fail(&e.to_string()),
                }
            }
            "copy" => {
                let text = cmd.path.unwrap_or_default();
                match ocr::wl_copy(&text) {
                    Ok(()) => emit(json!({"ok": true, "event": "copied"})),
                    Err(e) => fail(&e),
                }
            }
            "quit" => {
                request_stop();
            }
            other => fail(&format!("unknown cmd {other}")),
        }
        let _ = cmd.w;
        let _ = cmd.h;
        let _ = cmd.path;
        let _ = cmd.sentinel;
    }

    fn tick_stream(&mut self) {
        if !self.streaming {
            return;
        }
        if self.capturer.live_backend.pick_mode() {
            return;
        }
        match self.capture_live() {
            Ok(frame) => {
                let rect = self.last_frame_rect.unwrap_or(self.current_region());
                if let Err(e) = self.emit_frame(&frame, rect, self.last_frame_scale) {
                    fail(&e);
                }
            }
            Err(e) => fail(&e),
        }
    }
}

fn serve() {
    let mut server = Server::new();
    server.hello();
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let interval = Duration::from_millis(1000 / STREAM_HZ);
    let mut last = Instant::now();
    loop {
        if stop_requested() {
            break;
        }
        while let Ok(line) = rx.try_recv() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<Cmd>(line) {
                Ok(cmd) => server.handle(cmd),
                Err(e) => fail(&e.to_string()),
            }
            if stop_requested() {
                break;
            }
        }
        if stop_requested() {
            break;
        }
        if last.elapsed() >= interval {
            server.tick_stream();
            last = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    emit(json!({"ok": true, "event": "bye"}));
    remove_shm();
}

fn print_help() {
    println!(
        "chromad — Chroma capture helper\n\n\
         chromad --serve\n\
         chromad --capabilities\n\
         chromad --palette <image>\n\
         chromad --pixel <image> <x> <y>\n\
         chromad --calibrate --image <png|ppm> --layout <json>\n\
         chromad --map-coords --monitors <json> --x <n> --y <n>\n\
         chromad --check-recursion --image <png|ppm> --sentinel <hex>\n\
         chromad --kmeans <image>\n"
    );
}

fn arg_eq(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn arg_val(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || arg_eq(&args, "--help") || arg_eq(&args, "-h") {
        print_help();
        return;
    }
    if arg_eq(&args, "--serve") {
        serve();
        return;
    }
    if arg_eq(&args, "--capabilities") {
        let c = Capturer::negotiate();
        let (grim, ocr, qr) = ocr::which_report();
        println!(
            "{}",
            json!({
                "backend": c.live_backend.as_str(),
                "oneshotBackend": c.oneshot_backend.as_str(),
                "globals": c.globals,
                "live": c.live_backend.live(),
                "pickMode": c.live_backend.pick_mode(),
                "grim": grim,
                "ocr": ocr,
                "qr": qr
            })
        );
        return;
    }
    if arg_eq(&args, "--palette") || arg_eq(&args, "--kmeans") {
        let path = arg_val(&args, "--palette")
            .or_else(|| arg_val(&args, "--kmeans"))
            .or_else(|| args.get(1).cloned())
            .expect("image path");
        let frame = ppm::load_path(&path).expect("load image");
        let colors = kmeans_palette(frame.width, frame.height, &frame.rgba, 6);
        println!("{}", json!({"colors": nearest_hexes(&colors)}));
        return;
    }
    if arg_eq(&args, "--pixel") {
        let path = args.get(1).expect("image").clone();
        let x: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        let y: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
        let frame = ppm::load_path(&path).expect("load image");
        println!("{}", pixel_json(frame.pixel(x, y)));
        return;
    }
    if arg_eq(&args, "--calibrate") {
        let image = arg_val(&args, "--image").expect("--image");
        let layout_path = arg_val(&args, "--layout").expect("--layout");
        let frame = ppm::load_path(&image).expect("load image");
        let layout: calibrate::GridLayout =
            serde_json::from_str(&std::fs::read_to_string(layout_path).expect("layout")).expect("layout json");
        let results = calibrate::sample_grid(&frame, &layout);
        let ok = results.iter().all(|r| r.ok);
        let cells: Vec<_> = results
            .iter()
            .map(|r| {
                json!({
                    "row": r.row, "col": r.col,
                    "expected": r.expected, "got": r.got, "ok": r.ok
                })
            })
            .collect();
        println!("{}", json!({"ok": ok, "cells": cells}));
        std::process::exit(if ok { 0 } else { 2 });
    }
    if arg_eq(&args, "--map-coords") {
        let monitors = arg_val(&args, "--monitors").expect("--monitors");
        let x: f64 = arg_val(&args, "--x").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let y: f64 = arg_val(&args, "--y").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let mons = coords::parse_monitors(&std::fs::read_to_string(monitors).unwrap()).unwrap();
        let m = coords::containing(&mons, x, y).expect("monitor");
        let map = coords::to_physical(m, x, y);
        println!(
            "{}",
            json!({
                "monitor": map.monitor,
                "scale": map.scale,
                "physicalX": map.physical_x,
                "physicalY": map.physical_y
            })
        );
        return;
    }
    if arg_eq(&args, "--check-recursion") {
        let image = arg_val(&args, "--image").expect("--image");
        let sent = arg_val(&args, "--sentinel").unwrap_or_else(|| "#ff2bd6".into());
        let frame = ppm::load_path(&image).expect("load");
        let hit = calibrate::recursion_hit(&frame, Rgb::from_hex(&sent).unwrap_or(Rgb::new(255, 43, 214)), 4);
        println!("{}", json!({"hit": hit, "sentinel": sent}));
        std::process::exit(if hit { 3 } else { 0 });
    }
    print_help();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_constant_is_128() {
        assert_eq!(REGION, 128);
        let _ = crate::capture::Backend::Grim.as_str();
    }
}
