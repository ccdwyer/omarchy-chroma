use crate::coords::{self, Monitor, Rect};
use crate::grim;
use crate::ppm::Frame;
use std::env;

#[cfg(target_os = "linux")]
use crate::wayland;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    ExtCopy,
    WlrScreencopy,
    Grim,
    None,
}

impl Backend {
    pub fn as_str(self) -> &'static str {
        match self {
            Backend::ExtCopy => "ext-image-copy-capture-v1",
            Backend::WlrScreencopy => "wlr-screencopy",
            Backend::Grim => "grim",
            Backend::None => "none",
        }
    }

    pub fn live(self) -> bool {
        matches!(self, Backend::ExtCopy | Backend::WlrScreencopy)
    }

    pub fn pick_mode(self) -> bool {
        !self.live()
    }
}

pub struct Capturer {
    pub live_backend: Backend,
    pub oneshot_backend: Backend,
    pub globals: Vec<String>,
    #[cfg(target_os = "linux")]
    wayland: Option<wayland::WaylandCapture>,
}

impl Capturer {
    pub fn negotiate() -> Self {
        let force_grim = env::var("CHROMA_NO_PROTOCOL").ok().as_deref() == Some("1");
        #[allow(unused_mut)]
        let mut globals = Vec::new();
        #[cfg(target_os = "linux")]
        let wayland = if force_grim {
            None
        } else {
            match wayland::WaylandCapture::connect() {
                Ok(w) => {
                    globals = w.globals.clone();
                    Some(w)
                }
                Err(_) => None,
            }
        };
        #[cfg(not(target_os = "linux"))]
        let _ = force_grim;

        // Only advertise ExtCopy when both ext managers actually bound — a
        // global name in the registry is not enough if create_session is unused.
        #[cfg(target_os = "linux")]
        let (has_ext, has_wlr) = match &wayland {
            Some(w) => (w.has_ext(), w.has_wlr()),
            None => (false, false),
        };
        #[cfg(not(target_os = "linux"))]
        let (has_ext, has_wlr) = (false, false);
        let grim_ok = grim::available();
        let (live_backend, oneshot_backend) = choose_backends(force_grim, has_ext, has_wlr, grim_ok);

        Self {
            live_backend,
            oneshot_backend,
            globals,
            #[cfg(target_os = "linux")]
            wayland,
        }
    }

    /// `physical` is output-local buffer pixels. Callers keep the logical rect for QML.
    pub fn capture_on(&mut self, output: &str, physical: Rect, live: bool) -> Result<Frame, String> {
        let backend = if live {
            self.live_backend
        } else {
            self.oneshot_backend
        };
        match backend {
            Backend::ExtCopy | Backend::WlrScreencopy => {
                #[cfg(target_os = "linux")]
                {
                    if let Some(w) = self.wayland.as_mut() {
                        return w.capture_output(output, physical, backend == Backend::ExtCopy);
                    }
                }
                grim::capture_region_on(output, physical.x, physical.y, physical.w, physical.h)
            }
            Backend::Grim => {
                grim::capture_region_on(output, physical.x, physical.y, physical.w, physical.h)
            }
            Backend::None => Err("no capture backend".into()),
        }
    }

    pub fn capture_full(&mut self) -> Result<Frame, String> {
        match self.oneshot_backend {
            Backend::ExtCopy | Backend::WlrScreencopy => {
                #[cfg(target_os = "linux")]
                {
                    if let Some(w) = self.wayland.as_mut() {
                        return w.capture_full(self.oneshot_backend == Backend::ExtCopy);
                    }
                }
                grim::capture_full()
            }
            Backend::Grim => grim::capture_full(),
            Backend::None => Err("no capture backend".into()),
        }
    }
}

pub fn load_monitors() -> Vec<Monitor> {
    let out = std::process::Command::new("hyprctl")
        .args(["-j", "monitors"])
        .output();
    match out {
        Ok(o) if o.status.success() => coords::parse_monitors(&String::from_utf8_lossy(&o.stdout)).unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub fn load_cursor() -> Option<(f64, f64)> {
    if let Ok(o) = std::process::Command::new("hyprctl")
        .args(["-j", "cursorpos"])
        .output()
    {
        if o.status.success() {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                if let (Some(x), Some(y)) = (v.get("x").and_then(|n| n.as_f64()), v.get("y").and_then(|n| n.as_f64())) {
                    return Some((x, y));
                }
            }
        }
    }
    if let Ok(o) = std::process::Command::new("hyprctl").arg("cursorpos").output() {
        let s = String::from_utf8_lossy(&o.stdout);
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() == 2 {
            if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                return Some((x, y));
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ActiveWindow {
    pub at: (i32, i32),
    pub size: (i32, i32),
    pub monitor: Option<i64>,
}

pub fn load_active_window() -> Option<ActiveWindow> {
    let o = std::process::Command::new("hyprctl")
        .args(["-j", "activewindow"])
        .output()
        .ok()?;
    if !o.status.success() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).ok()?;
    let at = v.get("at")?.as_array()?;
    let size = v.get("size")?.as_array()?;
    Some(ActiveWindow {
        at: (
            at.first().and_then(|n| n.as_i64()).unwrap_or(0) as i32,
            at.get(1).and_then(|n| n.as_i64()).unwrap_or(0) as i32,
        ),
        size: (
            size.first().and_then(|n| n.as_i64()).unwrap_or(0) as i32,
            size.get(1).and_then(|n| n.as_i64()).unwrap_or(0) as i32,
        ),
        monitor: v.get("monitor").and_then(|n| n.as_i64()),
    })
}

pub fn choose_backends(force_grim: bool, has_ext: bool, has_wlr: bool, grim_ok: bool) -> (Backend, Backend) {
    // ExtCopy is only returned when the ext managers are bound. A registry
    // advertisement without a session/frame implementation must not win.
    let oneshot = if force_grim {
        if grim_ok {
            Backend::Grim
        } else {
            Backend::None
        }
    } else if has_ext {
        Backend::ExtCopy
    } else if has_wlr {
        Backend::WlrScreencopy
    } else if grim_ok {
        Backend::Grim
    } else {
        Backend::None
    };
    let live = if force_grim {
        if grim_ok {
            Backend::Grim
        } else {
            Backend::None
        }
    } else if has_wlr {
        Backend::WlrScreencopy
    } else if has_ext {
        Backend::ExtCopy
    } else if grim_ok {
        Backend::Grim
    } else {
        Backend::None
    };
    (live, oneshot)
}

pub fn window_rect_on_focused(monitors: &[Monitor]) -> Option<Rect> {
    let win = load_active_window()?;
    let focused = monitors.iter().find(|m| m.focused).or(monitors.first())?;
    let rect = Rect {
        x: win.at.0,
        y: win.at.1,
        w: win.size.0,
        h: win.size.1,
    };
    let clipped = coords::clamp_window_to_monitor(rect, focused);
    if clipped.is_empty() {
        None
    } else {
        Some(clipped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_only_compositor_selects_ext_for_oneshot_and_live() {
        let (live, oneshot) = choose_backends(false, true, false, true);
        assert_eq!(oneshot, Backend::ExtCopy);
        assert_eq!(live, Backend::ExtCopy);
    }

    #[test]
    fn never_select_ext_when_managers_were_not_bound() {
        let (live, oneshot) = choose_backends(false, false, true, true);
        assert_ne!(oneshot, Backend::ExtCopy);
        assert_eq!(oneshot, Backend::WlrScreencopy);
        assert_eq!(live, Backend::WlrScreencopy);
    }

    #[test]
    fn force_grim_never_advertises_ext() {
        let (live, oneshot) = choose_backends(true, true, true, true);
        assert_eq!(live, Backend::Grim);
        assert_eq!(oneshot, Backend::Grim);
    }

    #[test]
    fn no_backends_is_none() {
        let (live, oneshot) = choose_backends(false, false, false, false);
        assert_eq!(live, Backend::None);
        assert_eq!(oneshot, Backend::None);
    }
}
