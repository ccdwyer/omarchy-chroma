//! Linux-only capture: ext-image-copy-capture-v1, then wlr-screencopy.
//! Cursor is never composited (ext options empty / wlr overlay_cursor=0).

#![cfg(target_os = "linux")]

use crate::coords::Rect;
use crate::ppm::Frame;
use memmap2::MmapMut;
use rustix::fs::{memfd_create, MemfdFlags};
use std::os::fd::{AsFd, OwnedFd};
use wayland_client::protocol::{wl_buffer, wl_output, wl_registry, wl_shm, wl_shm_pool};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_image_capture_source_v1, ext_output_image_capture_source_manager_v1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1, ext_image_copy_capture_manager_v1,
    ext_image_copy_capture_session_v1,
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1, zwlr_screencopy_manager_v1,
};

struct OutputInfo {
    output: wl_output::WlOutput,
    name: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

struct ShmBuffer {
    _file: OwnedFd,
    map: MmapMut,
    pool: wl_shm_pool::WlShmPool,
    buffer: wl_buffer::WlBuffer,
    width: i32,
    height: i32,
    stride: i32,
    format: u32,
}

impl ShmBuffer {
    fn destroy(self) {
        self.buffer.destroy();
        self.pool.destroy();
    }
}

struct ExtGuards {
    source: Option<ext_image_capture_source_v1::ExtImageCaptureSourceV1>,
    session: Option<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1>,
    frame: Option<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1>,
    shm: Option<ShmBuffer>,
}

impl Drop for ExtGuards {
    fn drop(&mut self) {
        if let Some(frame) = self.frame.take() {
            frame.destroy();
        }
        if let Some(session) = self.session.take() {
            session.destroy();
        }
        if let Some(source) = self.source.take() {
            source.destroy();
        }
        if let Some(shm) = self.shm.take() {
            shm.destroy();
        }
    }
}

struct PendingFrame {
    width: i32,
    height: i32,
    stride: i32,
    format: u32,
    ready: bool,
    failed: bool,
}

struct ExtPending {
    width: i32,
    height: i32,
    formats: Vec<u32>,
    done: bool,
    ready: bool,
    failed: bool,
}

pub struct WaylandCapture {
    conn: Connection,
    qh: QueueHandle<State>,
    queue: wayland_client::EventQueue<State>,
    pub globals: Vec<String>,
    state: State,
}

struct State {
    globals: Vec<String>,
    shm: Option<wl_shm::WlShm>,
    outputs: Vec<OutputInfo>,
    screencopy: Option<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1>,
    ext_mgr: Option<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1>,
    ext_src: Option<ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1>,
    pending: Option<PendingFrame>,
    ext_pending: Option<ExtPending>,
}

impl WaylandCapture {
    pub fn connect() -> Result<Self, String> {
        let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        let display = conn.display();
        let _registry = display.get_registry(&qh, ());
        let mut state = State {
            globals: Vec::new(),
            shm: None,
            outputs: Vec::new(),
            screencopy: None,
            ext_mgr: None,
            ext_src: None,
            pending: None,
            ext_pending: None,
        };
        queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
        if state.shm.is_none() {
            return Err("wl_shm missing".into());
        }
        let has_ext = state.ext_mgr.is_some() && state.ext_src.is_some();
        if state.screencopy.is_none() && !has_ext {
            return Err("no screencopy global".into());
        }
        let globals = state.globals.clone();
        Ok(Self {
            conn,
            qh,
            queue,
            globals,
            state,
        })
    }

    pub fn has_ext(&self) -> bool {
        self.state.ext_mgr.is_some() && self.state.ext_src.is_some()
    }

    pub fn has_wlr(&self) -> bool {
        self.state.screencopy.is_some()
    }

    fn output_named(&self, name: &str) -> Option<&OutputInfo> {
        self.state
            .outputs
            .iter()
            .find(|o| o.name == name)
            .or(self.state.outputs.first())
    }

    fn output_for_rect(&self, rect: Rect) -> Option<&OutputInfo> {
        self.state
            .outputs
            .iter()
            .find(|o| {
                rect.x >= o.x
                    && rect.y >= o.y
                    && rect.x < o.x + o.width.max(1)
                    && rect.y < o.y + o.height.max(1)
            })
            .or(self.state.outputs.first())
    }

    pub fn capture_region(&mut self, rect: Rect, prefer_ext: bool) -> Result<Frame, String> {
        let name = self
            .output_for_rect(rect)
            .map(|o| o.name.clone())
            .unwrap_or_default();
        self.capture_output(&name, rect, prefer_ext)
    }

    pub fn capture_full(&mut self, prefer_ext: bool) -> Result<Frame, String> {
        let out = self
            .state
            .outputs
            .first()
            .ok_or_else(|| "no wl_output".to_string())?;
        let name = out.name.clone();
        let rect = Rect {
            x: out.x,
            y: out.y,
            w: out.width.max(1),
            h: out.height.max(1),
        };
        self.capture_output(&name, rect, prefer_ext)
    }

    pub fn capture_output(
        &mut self,
        name: &str,
        rect: Rect,
        prefer_ext: bool,
    ) -> Result<Frame, String> {
        if prefer_ext && self.has_ext() {
            return self.ext_output(name, rect);
        }
        if self.has_wlr() {
            match self.wlr_output(name, rect) {
                Ok(f) => return Ok(f),
                Err(e) if self.has_ext() => {
                    return self.ext_output(name, rect).map_err(|e2| format!("{e}; ext: {e2}"))
                }
                Err(e) => return Err(e),
            }
        }
        if self.has_ext() {
            return self.ext_output(name, rect);
        }
        Err("no wayland capture backend bound".into())
    }

    fn ext_output(&mut self, name: &str, rect: Rect) -> Result<Frame, String> {
        let mgr = self
            .state
            .ext_mgr
            .as_ref()
            .ok_or_else(|| "ext_image_copy_capture_manager_v1 missing".to_string())?
            .clone();
        let src_mgr = self
            .state
            .ext_src
            .as_ref()
            .ok_or_else(|| "ext_output_image_capture_source_manager_v1 missing".to_string())?
            .clone();
        let output = self
            .output_named(name)
            .ok_or_else(|| format!("output {name} missing"))?
            .output
            .clone();
        let out_x = self.output_named(name).map(|o| o.x).unwrap_or(0);
        let out_y = self.output_named(name).map(|o| o.y).unwrap_or(0);

        self.state.ext_pending = Some(ExtPending {
            width: 0,
            height: 0,
            formats: Vec::new(),
            done: false,
            ready: false,
            failed: false,
        });
        // paint_cursors is unset so the cursor sprite is not in the sample.
        let source = src_mgr.create_source(&output, &self.qh, ());
        let session = mgr.create_session(
            &source,
            ext_image_copy_capture_manager_v1::Options::empty(),
            &self.qh,
            (),
        );
        let mut guards = ExtGuards {
            source: Some(source),
            session: Some(session),
            frame: None,
            shm: None,
        };
        let mut spins = 0;
        while spins < 48 {
            self.queue
                .roundtrip(&mut self.state)
                .map_err(|e| e.to_string())?;
            if self
                .state
                .ext_pending
                .as_ref()
                .map(|p| p.done || p.failed)
                .unwrap_or(false)
            {
                break;
            }
            spins += 1;
        }
        let (width, height, format) = {
            let p = self
                .state
                .ext_pending
                .as_ref()
                .ok_or_else(|| "ext session produced no constraints".to_string())?;
            if p.failed {
                return Err("ext-image-copy-capture session failed".into());
            }
            if !p.done || p.width <= 0 || p.height <= 0 {
                return Err("ext-image-copy-capture offered no buffer".into());
            }
            let fmt = pick_shm_format(&p.formats);
            (p.width, p.height, fmt)
        };
        let stride = width * 4;
        let shm_buf = make_shm_buffer(
            self.state.shm.as_ref().unwrap(),
            &self.qh,
            width,
            height,
            stride,
            format,
        )?;
        if let Some(p) = self.state.ext_pending.as_mut() {
            p.ready = false;
            p.failed = false;
        }
        let frame = guards
            .session
            .as_ref()
            .ok_or_else(|| "ext session missing".to_string())?
            .create_frame(&self.qh, ());
        frame.attach_buffer(&shm_buf.buffer);
        frame.capture();
        guards.frame = Some(frame);
        guards.shm = Some(shm_buf);
        spins = 0;
        while spins < 48 {
            self.queue
                .roundtrip(&mut self.state)
                .map_err(|e| e.to_string())?;
            if self
                .state
                .ext_pending
                .as_ref()
                .map(|p| p.ready || p.failed)
                .unwrap_or(false)
            {
                break;
            }
            spins += 1;
        }
        if self
            .state
            .ext_pending
            .as_ref()
            .map(|p| p.failed)
            .unwrap_or(false)
        {
            return Err("ext-image-copy-capture copy failed".into());
        }
        if !self
            .state
            .ext_pending
            .as_ref()
            .map(|p| p.ready)
            .unwrap_or(false)
        {
            return Err("ext-image-copy-capture timed out".into());
        }
        let shm_ref = guards
            .shm
            .as_ref()
            .ok_or_else(|| "ext shm buffer missing".to_string())?;
        let rgba = shm_to_rgba(shm_ref)?;
        let full = Frame {
            width: width as u32,
            height: height as u32,
            rgba,
        };
        let local_x = (rect.x - out_x).max(0);
        let local_y = (rect.y - out_y).max(0);
        let _ = self.conn;
        drop(guards);
        if local_x == 0 && local_y == 0 && rect.w >= width && rect.h >= height {
            Ok(full)
        } else {
            Ok(full.crop(local_x, local_y, rect.w.max(1) as u32, rect.h.max(1) as u32))
        }
    }

    fn wlr_output(&mut self, name: &str, rect: Rect) -> Result<Frame, String> {
        let manager = self
            .state
            .screencopy
            .as_ref()
            .ok_or_else(|| "zwlr_screencopy_manager_v1 missing".to_string())?
            .clone();
        let output = self
            .output_named(name)
            .ok_or_else(|| format!("output {name} missing"))?
            .output
            .clone();
        let out_x = self.output_named(name).map(|o| o.x).unwrap_or(0);
        let out_y = self.output_named(name).map(|o| o.y).unwrap_or(0);
        let local_x = (rect.x - out_x).max(0);
        let local_y = (rect.y - out_y).max(0);
        self.state.pending = Some(PendingFrame {
            width: 0,
            height: 0,
            stride: 0,
            format: 0,
            ready: false,
            failed: false,
        });
        let _probe = manager.capture_output_region(
            0,
            &output,
            local_x,
            local_y,
            rect.w.max(1),
            rect.h.max(1),
            &self.qh,
            (),
        );
        self.queue
            .roundtrip(&mut self.state)
            .map_err(|e| e.to_string())?;
        _probe.destroy();
        let (width, height, stride, format) = {
            let p = self
                .state
                .pending
                .as_ref()
                .ok_or_else(|| "no buffer event".to_string())?;
            if p.failed {
                return Err("screencopy failed".into());
            }
            if p.width <= 0 || p.height <= 0 {
                return Err("screencopy offered no buffer".into());
            }
            (p.width, p.height, p.stride, p.format)
        };
        let shm_buf = make_shm_buffer(
            self.state.shm.as_ref().unwrap(),
            &self.qh,
            width,
            height,
            stride,
            format,
        )?;
        let frame = manager.capture_output_region(
            0,
            &output,
            local_x,
            local_y,
            rect.w.max(1),
            rect.h.max(1),
            &self.qh,
            (),
        );
        self.queue
            .roundtrip(&mut self.state)
            .map_err(|e| e.to_string())?;
        frame.copy(&shm_buf.buffer);
        self.state.pending = Some(PendingFrame {
            width,
            height,
            stride,
            format,
            ready: false,
            failed: false,
        });
        let mut spins = 0;
        let mut copy_err: Option<String> = None;
        while spins < 32 {
            if let Err(e) = self.queue.roundtrip(&mut self.state) {
                copy_err = Some(e.to_string());
                break;
            }
            if self.state.pending.as_ref().map(|p| p.failed).unwrap_or(false) {
                copy_err = Some("screencopy copy failed".into());
                break;
            }
            if self.state.pending.as_ref().map(|p| p.ready).unwrap_or(false) {
                break;
            }
            spins += 1;
        }
        if let Some(e) = copy_err {
            frame.destroy();
            shm_buf.destroy();
            return Err(e);
        }
        let rgba = shm_to_rgba(&shm_buf)?;
        frame.destroy();
        shm_buf.destroy();
        Ok(Frame {
            width: width as u32,
            height: height as u32,
            rgba,
        })
    }
}

fn pick_shm_format(formats: &[u32]) -> u32 {
    const ARGB: u32 = 0;
    const XRGB: u32 = 1;
    if formats.iter().any(|f| *f == ARGB) {
        return ARGB;
    }
    if formats.iter().any(|f| *f == XRGB) {
        return XRGB;
    }
    formats.first().copied().unwrap_or(ARGB)
}

fn make_shm_buffer(
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<State>,
    width: i32,
    height: i32,
    stride: i32,
    format: u32,
) -> Result<ShmBuffer, String> {
    let size = (stride * height).max(stride) as usize;
    let fd = memfd_create("chroma-shm", MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING)
        .map_err(|e| e.to_string())?;
    rustix::fs::ftruncate(&fd, size as u64).map_err(|e| e.to_string())?;
    let map = unsafe { MmapMut::map_mut(&fd) }.map_err(|e| e.to_string())?;
    let pool = shm.create_pool(fd.as_fd(), size as i32, qh, ());
    let wl_format = match format {
        0 => wl_shm::Format::Argb8888,
        1 => wl_shm::Format::Xrgb8888,
        0x34325241 => wl_shm::Format::Argb8888,
        0x34325258 => wl_shm::Format::Xrgb8888,
        _ => wl_shm::Format::Argb8888,
    };
    let buffer = pool.create_buffer(0, width, height, stride, wl_format, qh, ());
    Ok(ShmBuffer {
        _file: fd,
        map,
        pool,
        buffer,
        width,
        height,
        stride,
        format,
    })
}

fn shm_to_rgba(buf: &ShmBuffer) -> Result<Vec<u8>, String> {
    let w = buf.width as usize;
    let h = buf.height as usize;
    let stride = buf.stride as usize;
    let mut rgba = vec![0u8; w * h * 4];
    let src = &buf.map[..];
    for y in 0..h {
        for x in 0..w {
            let i = y * stride + x * 4;
            if i + 3 >= src.len() {
                continue;
            }
            let (r, g, b, a) = match buf.format {
                0 | 1 | 0x34325241 | 0x34325258 => (src[i + 2], src[i + 1], src[i], src[i + 3]),
                _ => (src[i], src[i + 1], src[i + 2], src[i + 3]),
            };
            let o = (y * w + x) * 4;
            rgba[o] = r;
            rgba[o + 1] = g;
            rgba[o + 2] = b;
            rgba[o + 3] = if buf.format == 1 || buf.format == 0x34325258 {
                255
            } else {
                a
            };
        }
    }
    Ok(rgba)
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            state.globals.push(interface.clone());
            match interface.as_str() {
                "wl_shm" => {
                    state.shm = Some(registry.bind::<wl_shm::WlShm, _, _>(name, version.min(1), qh, ()));
                }
                "wl_output" => {
                    let output =
                        registry.bind::<wl_output::WlOutput, _, _>(name, version.min(4), qh, ());
                    state.outputs.push(OutputInfo {
                        output,
                        name: format!("output-{name}"),
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                    });
                }
                "zwlr_screencopy_manager_v1" => {
                    state.screencopy = Some(registry.bind::<
                        zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
                        _,
                        _,
                    >(name, version.min(3), qh, ()));
                }
                "ext_image_copy_capture_manager_v1" => {
                    state.ext_mgr = Some(registry.bind::<
                        ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
                        _,
                        _,
                    >(name, version.min(1), qh, ()));
                }
                "ext_output_image_capture_source_manager_v1" => {
                    state.ext_src = Some(registry.bind::<
                        ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
                        _,
                        _,
                    >(name, version.min(1), qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_shm::WlShm, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_shm::WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_shm_pool::WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_buffer::WlBuffer,
        _: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_output::WlOutput, ()> for State {
    fn event(
        state: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(info) = state.outputs.iter_mut().find(|o| o.output == *output) else {
            return;
        };
        match event {
            wl_output::Event::Geometry { x, y, .. } => {
                info.x = x;
                info.y = y;
            }
            wl_output::Event::Mode {
                flags,
                width,
                height,
                ..
            } => {
                let current = match flags {
                    WEnum::Value(f) => f.contains(wl_output::Mode::Current),
                    _ => true,
                };
                if current {
                    info.width = width;
                    info.height = height;
                }
            }
            wl_output::Event::Name { name } => info.name = name,
            _ => {}
        }
    }
}

impl Dispatch<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
        _: zwlr_screencopy_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                let fmt = match format {
                    WEnum::Value(wl_shm::Format::Argb8888) => 0,
                    WEnum::Value(wl_shm::Format::Xrgb8888) => 1,
                    WEnum::Value(other) => other as u32,
                    WEnum::Unknown(u) => u,
                };
                if let Some(p) = state.pending.as_mut() {
                    p.width = width as i32;
                    p.height = height as i32;
                    p.stride = stride as i32;
                    p.format = fmt;
                } else {
                    state.pending = Some(PendingFrame {
                        width: width as i32,
                        height: height as i32,
                        stride: stride as i32,
                        format: fmt,
                        ready: false,
                        failed: false,
                    });
                }
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                if let Some(p) = state.pending.as_mut() {
                    p.ready = true;
                }
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                if let Some(p) = state.pending.as_mut() {
                    p.failed = true;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
        _: ext_image_copy_capture_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1, ()>
    for State
{
    fn event(
        _: &mut Self,
        _: &ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        _: ext_output_image_capture_source_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_image_capture_source_v1::ExtImageCaptureSourceV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ext_image_capture_source_v1::ExtImageCaptureSourceV1,
        _: ext_image_capture_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                if let Some(p) = state.ext_pending.as_mut() {
                    p.width = width as i32;
                    p.height = height as i32;
                }
            }
            ext_image_copy_capture_session_v1::Event::ShmFormat { format } => {
                let fmt = match format {
                    WEnum::Value(wl_shm::Format::Argb8888) => 0,
                    WEnum::Value(wl_shm::Format::Xrgb8888) => 1,
                    WEnum::Value(other) => other as u32,
                    WEnum::Unknown(u) => u,
                };
                if let Some(p) = state.ext_pending.as_mut() {
                    p.formats.push(fmt);
                }
            }
            ext_image_copy_capture_session_v1::Event::Done => {
                if let Some(p) = state.ext_pending.as_mut() {
                    p.done = true;
                }
            }
            ext_image_copy_capture_session_v1::Event::Stopped => {
                if let Some(p) = state.ext_pending.as_mut() {
                    p.failed = true;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_image_copy_capture_frame_v1::Event::Ready => {
                if let Some(p) = state.ext_pending.as_mut() {
                    p.ready = true;
                }
            }
            ext_image_copy_capture_frame_v1::Event::Failed { .. } => {
                if let Some(p) = state.ext_pending.as_mut() {
                    p.failed = true;
                }
            }
            _ => {}
        }
    }
}
