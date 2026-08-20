# Assumptions

Conservative choices where the Omarchy / Quickshell / Hyprland API was not 100% certain. Isolated behind small adapters (`qml/ChromadClient.qml`, `src/chromad/src/capture.rs`, `js/Capture.js`). Where this file and `docs/07-chroma.md` disagree, this file plus `docs/quattro-shell-reference.md` win.

## Plugin host (reference wins)

- **Manifest** is schemaVersion 1 with `kinds: ["bar-widget","overlay"]`, `keepLoaded: true`, and a `barWidget` metadata block (`displayName`, `category`, `defaultSection`, `defaults`, `schema`). Settings arrive **inline on the shell.json entry** — there is no plugin `config.json` for widget settings. Pick history is runtime state, not a setting.
- **Entry points are `Item`s**, not `ShellRoot`. Overlay exposes `open(payloadJson)` / `close()` / `toggle()` plus `pick(arg)` / `palette(arg)` / `revert(arg)` / `status(arg)` (string in, string out) on the **root Item**. `omarchy-shell shell call <id> <method> <arg>` invokes those on the overlay loader (`keepLoaded`). The overlay `IpcHandler` with the same id is the reliable bind path: `omarchy-shell io.github.chris.chroma pick '{}'`. Every IpcHandler method takes a string argument (empty when unused).
- **`keepLoaded: true`** so the overlay (and the chromad Process) outlives a summon, matching image-picker.
- **Injected properties** on load: `shell`, `manifest`, `pluginRegistry`, `omarchyPath` (and `bar` on the widget). Overlay still functions if some are missing.
- **IPC verbs** are `omarchy-shell shell summon|hide|toggle` for the overlay, plus `omarchy-shell io.github.chris.chroma <method> <arg>` for the overlay IpcHandler. `shell call` is extra while the overlay is loaded. First-party plugins use short names; a unique id avoids collisions.
- **Third-party id** is `io.github.chris.chroma` — not `omarchy.*`.
- **No second Quickshell process.**

## Interaction model (frozen)

- Spec required a day-1 spike of **Quickshell’s** input-region API, not just the protocol. `PanelWindow` has a `mask` / `Region` in some builds; it is **not** documented as clearly as `WlrLayershell.keyboardFocus`. We did **not** depend on click-through. The overlay takes `WlrKeyboardFocus.Exclusive`. Keyboard is the primary path (and the accessibility path). HUD buttons still accept clicks. Space picks.
- Cursor is **not** assumed to be a Quickshell property. Overlay polls `hyprctl -j cursorpos` at 16ms (≤60Hz), with a text `x, y` fallback.

## Capture

- **Negotiation order** matches the spec for oneshots: ext-image-copy-capture-v1, then wlr-screencopy, then grim. **ExtCopy is selected only when both `ext_image_copy_capture_manager_v1` and `ext_output_image_capture_source_manager_v1` actually bind.** `wayland.rs` implements the session/frame path (create_source → create_session with empty options so the cursor is not painted → attach_buffer → capture, then crop). An ext-only compositor no longer gets a backend name that then calls wlr.
- **Live 128×128 @ 30Hz** prefers `zwlr_screencopy_manager_v1.capture_output_region` (`overlay_cursor=0`) when that global is bound; otherwise it uses the ext session and crops. hello.backend / oneshotBackend follow those rules.
- **grim is pick-mode**, never a live slideshow. HUD says so. `CHROMA_NO_PROTOCOL=1` forces it.
- **Self-capture:** loupe offset (~60px), cursor excluded, overlay unmapped then `hyprctl -j layers` polled until the `chroma` namespace is gone (oneshots wait for compositor acknowledgement, not a fixed 34ms), freeze-frame zoom.
- **Frame transport:** two files under `/dev/shm/chroma-<pid>/` (else `$XDG_RUNTIME_DIR`, else tmp) plus `?n=` cache-bust. If a `file://` query string makes Qt refuse the path, the slot filename still changes (`frame0.png` / `frame1.png`).
- **QML Image `sourceClipRect`** is used for local crop/zoom. ShaderEffect was not required.
- Wayland protocol code is `cfg(target_os = "linux")`. On this macOS machine it is not linked. **No Linux ELF is committed** (there is no cross-toolchain here; fake binaries are forbidden). `.github/workflows/chromad-linux.yml` builds x86_64 (native) and aarch64 (gcc-aarch64-linux-gnu) on Ubuntu, writes `SHA256SUMS`, and attaches them to GitHub Releases. `scripts/fetch-prebuilts.sh` installs those assets into `bin/chromad`. `build.sh` remains the source path. Missing binary → grim fallback.
- Coordinate mapping treats `hyprctl -j monitors` `width`/`height` as compositor-reported pixels and `scale` as the factor. Logical size = width/scale. **Wayland capture uses output-local physical pixels** (`MappedCapture.physical`). **grim uses compositor-layout geometry** (`MappedCapture.layout`, the same space as slurp / xdg-output / `grim -g`) and requires a resolved output name — no silent first-output fallback. `wayland.rs` does a second registry roundtrip so output name/geometry/mode are delivered before capture. Frame events still carry the **logical** `x,y,w,h` plus `scale` so QML `sourceClipRect` can map cursor micro-movement onto the physical buffer. `chromad --calibrate` remains the on-device gate.

## Theme apply

- **Do not assume `omarchy theme set`.** Apply is `omarchy-theme-set <name>` — the script the desktop actually runs (writes `current/theme`, `theme.name`, templates, restarts). Fallback is not invented.
- Preview directory is `~/.config/omarchy/themes/chroma-preview/` with `colors.toml` (required keys matching shipped tokyo-night / catppuccin) plus minimal `hyprland.conf` / `alacritty.toml` if templates are absent.
- Transaction: when `themeLive` is false, snapshot `theme.name` at the start of **every** preview (overwrite or clear the session record — empty/`chroma-preview` names do not leave a stale original). Application is refused unless a valid, non-preview revert target has been persisted. Then write files → `theme_valid` → `omarchy-theme-set chroma-preview` → **`themeLive` only if that process exits 0**. A successful revert clears `originalTheme` and the session file so the next preview cannot restore a stale theme. Mapping lives in `js/ThemeSession.js`.
- Mapping: darkest → background, contrast-fixed lightest → foreground, highest chroma among remaining → accent. Validated against tokyo-night and catppuccin **token files** off-device (not a live apply).

## Quickshell types actually used

`PanelWindow`, `WlrLayershell` / `WlrLayer.Overlay` / `WlrKeyboardFocus.Exclusive` / `ExclusionMode.Ignore`, `Process` (`stdinEnabled`, `write`, `SplitParser`, `StdioCollector`), `FileView` (`setText`, `atomicWrites`), `IpcHandler`, `Hyprland` is **not** required (hyprctl Process instead), `BarWidget`, `WidgetButton`, `BorderSurface`, `Color.*`, `Style.*`, `Border.*`.

`Quickshell.execDetached` is used only as a summon fallback from the bar chip.

Clipboard is `wl-copy` via `Process`, not an undocumented `Quickshell.clipboard`.

## Helper fallback

Missing `bin/chromad` → `compat/chromad.sh` (grim oneshots). Palette from `source=window` parses `hyprctl -j activewindow` ∩ focused monitor and passes that compositor-layout geometry to grim. Palette from `source=monitor` captures the **focused output** (`grim -o`), never the entire desktop. Region geometry is output-aware and keeps signed coordinates (monitors left/above layout origin). Pixel reads require a real PPM parse (python3); missing Python or a parse failure is a protocol error — never `#808080`. Without python3, palette returns an **error** (no invented swatches). `freeze` emits `frame` + `frozen` only. Frame dirs are removed after the serve loop ends: SIGINT/SIGTERM set `STOP` and let the loop exit; `EXIT` removes the shm directory. OCR/QR hide when `tesseract` / `zbarimg` are absent.

Linux prebuilts: `scripts/fetch-prebuilts.sh` keeps the published asset filename, verifies it against `SHA256SUMS` (fail closed), then installs as `bin/chromad`.

## Review artifacts

`.gpt_review_err.log`, `.gpt_review_r1.md`, `.review_prompt.md` (and similar) stay on disk for the review gate. They are gitignored so they are not packaged. They are **not** deleted.

## Out of scope (intentional)

- Persistent guides (v1.1)
- Millimetre ruler
- A C++ QImage wrapper (plugins cannot reliably register C++ types in the shared shell)
- Writing Hyprland config
- Network, accounts, telemetry
