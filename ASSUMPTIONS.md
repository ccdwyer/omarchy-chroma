# Assumptions

Conservative choices where the Omarchy / Quickshell / Hyprland API was not 100% certain. Isolated behind small adapters (`qml/ChromadClient.qml`, `src/chromad/src/capture.rs`, `js/Capture.js`). Where this file and `docs/07-chroma.md` disagree, this file plus `docs/quattro-shell-reference.md` win.

## Plugin host (reference wins)

- **Manifest** is schemaVersion 1 with `kinds: ["bar-widget","overlay"]`, `keepLoaded: true`, and a `barWidget` metadata block (`displayName`, `category`, `defaultSection`, `defaults`, `schema`). Settings arrive **inline on the shell.json entry** — there is no plugin `config.json` for widget settings. Pick history is runtime state, not a setting.
- **Entry points are `Item`s**, not `ShellRoot`. Overlay exposes `open(payloadJson)` / `close()` / `toggle()` for `omarchy-shell shell summon|hide|toggle|call <id> …`.
- **`keepLoaded: true`** so the overlay (and the chromad Process) outlives a summon, matching image-picker.
- **Injected properties** on load: `shell`, `manifest`, `pluginRegistry`, `omarchyPath` (and `bar` on the widget). Overlay still functions if some are missing.
- **IPC verbs** are `omarchy-shell shell summon|hide|toggle|call`. `IpcHandler` target is the plugin id. First-party plugins use short names; a unique id avoids collisions.
- **Third-party id** is `io.github.chris.chroma` — not `omarchy.*`.
- **No second Quickshell process.**

## Interaction model (frozen)

- Spec required a day-1 spike of **Quickshell’s** input-region API, not just the protocol. `PanelWindow` has a `mask` / `Region` in some builds; it is **not** documented as clearly as `WlrLayershell.keyboardFocus`. We did **not** depend on click-through. The overlay takes `WlrKeyboardFocus.Exclusive`. Keyboard is the primary path (and the accessibility path). HUD buttons still accept clicks. Space picks.
- Cursor is **not** assumed to be a Quickshell property. Overlay polls `hyprctl -j cursorpos` at 16ms (≤60Hz), with a text `x, y` fallback.

## Capture

- **Negotiation order** matches the spec for oneshots: ext-image-copy-capture-v1, then wlr-screencopy, then grim.
- **Live 128×128 @ 30Hz** uses `zwlr_screencopy_manager_v1.capture_output_region` with `overlay_cursor=0` when that global exists. ext-image-copy-capture is a full-source session; using it for the live crop would pull a whole output every tick. Deviation is recorded: live region prefers wlr-screencopy; hello.backend / oneshotBackend are reported separately.
- **grim is pick-mode**, never a live slideshow. HUD says so. `CHROMA_NO_PROTOCOL=1` forces it.
- **Self-capture:** loupe offset (~60px), cursor excluded, overlay hidden ~34ms for oneshots, freeze-frame zoom.
- **Frame transport:** two files under `/dev/shm/chroma-<pid>/` (else `$XDG_RUNTIME_DIR`, else tmp) plus `?n=` cache-bust. If a `file://` query string makes Qt refuse the path, the slot filename still changes (`frame0.png` / `frame1.png`).
- **QML Image `sourceClipRect`** is used for local crop/zoom. ShaderEffect was not required.
- Wayland protocol code is `cfg(target_os = "linux")`. On this macOS machine it is not linked. `build.sh` produces a host binary; Linux x86_64/aarch64 prebuilts are produced by running `build.sh` on the target (or CI), not here.
- Coordinate mapping treats `hyprctl -j monitors` `width`/`height` as compositor-reported pixels and `scale` as the factor. Logical size = width/scale; physical = (logical − origin) × scale. `chromad --calibrate` is the gate; 1x/1.5x/2x nested Hyprland cannot be run here.

## Theme apply

- **Do not assume `omarchy theme set`.** Apply is `omarchy-theme-set <name>` — the script the desktop actually runs (writes `current/theme`, `theme.name`, templates, restarts). Fallback is not invented.
- Preview directory is `~/.config/omarchy/themes/chroma-preview/` with `colors.toml` (required keys matching shipped tokyo-night / catppuccin) plus minimal `hyprland.conf` / `alacritty.toml` if templates are absent.
- Transaction: snapshot `theme.name` → write files → validate keys → apply → revert always offered. If apply fails, the original name is still snapshotted for `u`.
- Mapping: darkest → background, contrast-fixed lightest → foreground, highest chroma among remaining → accent. Validated against tokyo-night and catppuccin **token files** off-device (not a live apply).

## Quickshell types actually used

`PanelWindow`, `WlrLayershell` / `WlrLayer.Overlay` / `WlrKeyboardFocus.Exclusive` / `ExclusionMode.Ignore`, `Process` (`stdinEnabled`, `write`, `SplitParser`, `StdioCollector`), `FileView` (`setText`, `atomicWrites`), `IpcHandler`, `Hyprland` is **not** required (hyprctl Process instead), `BarWidget`, `WidgetButton`, `BorderSurface`, `Color.*`, `Style.*`, `Border.*`.

`Quickshell.execDetached` is used only as a summon fallback from the bar chip.

Clipboard is `wl-copy` via `Process`, not an undocumented `Quickshell.clipboard`.

## Helper fallback

Missing `bin/chromad` → `compat/chromad.sh` (grim oneshots + python3 k-means if present). Core picker/palette/theme survive with zero Rust binary. OCR/QR hide when `tesseract` / `zbarimg` are absent.

## Out of scope (intentional)

- Persistent guides (v1.1)
- Millimetre ruler
- A C++ QImage wrapper (plugins cannot reliably register C++ types in the shared shell)
- Writing Hyprland config
- Network, accounts, telemetry
