# Assumptions

Conservative choices where the Omarchy / Quickshell / Hyprland API was not 100% certain. Isolated behind small adapters (`qml/ChromadClient.qml`, `src/chromad/src/capture.rs`, `js/Capture.js`). Where this file and `docs/07-chroma.md` disagree, this file plus `docs/quattro-shell-reference.md` win.

## Plugin host (reference wins)

- **Manifest** is schemaVersion 1 with `kinds: ["bar-widget","overlay"]`, `keepLoaded: true`, and a `barWidget` metadata block (`displayName`, `category`, `defaultSection`, `defaults`, `schema`). Settings arrive **inline on the shell.json entry** — there is no plugin `config.json` for widget settings. Pick history is runtime state, not a setting.
- **Entry points are `Item`s**, not `ShellRoot`. Overlay exposes `open(payloadJson)` / `close()` / `toggle()` plus `pick(arg)` / `palette(arg)` / `revert(arg)` / `status(arg)` (string in, string out) on the **root Item**, which is what `omarchy-shell shell call <id> <method> <arg>` invokes. `IpcHandler` with the same id forwards to those functions.
- **`keepLoaded: true`** so the overlay (and the chromad Process) outlives a summon, matching image-picker.
- **Injected properties** on load: `shell`, `manifest`, `pluginRegistry`, `omarchyPath` (and `bar` on the widget). Overlay still functions if some are missing.
- **IPC verbs** are `omarchy-shell shell summon|hide|toggle|call`. `IpcHandler` target is the plugin id. First-party plugins use short names; a unique id avoids collisions.
- **Third-party id** is `io.github.chris.chroma` — not `omarchy.*`.
- **No second Quickshell process.**

## Interaction model (frozen)

- Spec required a day-1 spike of **Quickshell’s** input-region API, not just the protocol. `PanelWindow` has a `mask` / `Region` in some builds; it is **not** documented as clearly as `WlrLayershell.keyboardFocus`. We did **not** depend on click-through. The overlay takes `WlrKeyboardFocus.Exclusive`. Keyboard is the primary path (and the accessibility path). HUD buttons still accept clicks. Space picks.
- Cursor is **not** assumed to be a Quickshell property. Overlay polls `hyprctl -j cursorpos` at 16ms (≤60Hz), with a text `x, y` fallback.

## Capture

- **Negotiation order** matches the spec for oneshots: ext-image-copy-capture-v1, then wlr-screencopy, then grim. **ExtCopy is selected only when both `ext_image_copy_capture_manager_v1` and `ext_output_image_capture_source_manager_v1` actually bind.** `wayland.rs` implements the session/frame path (create_source → create_session with empty options so the cursor is not painted → attach_buffer → capture, then crop). An ext-only compositor no longer gets a backend name that then calls wlr.
- **Live 128×128 @ 30Hz** prefers `zwlr_screencopy_manager_v1.capture_output_region` (`overlay_cursor=0`) when that global is bound; otherwise it uses the ext session and crops. hello.backend / oneshotBackend follow those rules.
- **grim is pick-mode**, never a live slideshow. HUD says so. `CHROMA_NO_PROTOCOL=1` forces it.
- **Self-capture:** loupe offset (~60px), cursor excluded, overlay hidden ~34ms for oneshots, freeze-frame zoom.
- **Frame transport:** two files under `/dev/shm/chroma-<pid>/` (else `$XDG_RUNTIME_DIR`, else tmp) plus `?n=` cache-bust. If a `file://` query string makes Qt refuse the path, the slot filename still changes (`frame0.png` / `frame1.png`).
- **QML Image `sourceClipRect`** is used for local crop/zoom. ShaderEffect was not required.
- Wayland protocol code is `cfg(target_os = "linux")`. On this macOS machine it is not linked. **No Linux ELF is committed** (there is no cross-toolchain here; fake binaries are forbidden). `.github/workflows/chromad-linux.yml` builds x86_64 (native) and aarch64 (gcc-aarch64-linux-gnu) on Ubuntu, writes `SHA256SUMS`, and attaches them to GitHub Releases. `scripts/fetch-prebuilts.sh` installs those assets into `bin/chromad`. `build.sh` remains the source path. Missing binary → grim fallback.
- Coordinate mapping treats `hyprctl -j monitors` `width`/`height` as compositor-reported pixels and `scale` as the factor. Logical size = width/scale; physical = (logical − origin) × scale. `chromad --calibrate` is the gate; 1x/1.5x/2x nested Hyprland cannot be run here.

## Theme apply

- **Do not assume `omarchy theme set`.** Apply is `omarchy-theme-set <name>` — the script the desktop actually runs (writes `current/theme`, `theme.name`, templates, restarts). Fallback is not invented.
- Preview directory is `~/.config/omarchy/themes/chroma-preview/` with `colors.toml` (required keys matching shipped tokyo-night / catppuccin) plus minimal `hyprland.conf` / `alacritty.toml` if templates are absent.
- Transaction: snapshot `theme.name` → write files → `theme_valid` from chromad (or local JS validate if chromad is down) → `omarchy-theme-set chroma-preview` → **`themeLive` (revert UI) only if that process exits 0**. Failure leaves `themeLive` false. Revert waits for `omarchy-theme-set <original>` exit 0 before clearing `themeLive`.
- Mapping: darkest → background, contrast-fixed lightest → foreground, highest chroma among remaining → accent. Validated against tokyo-night and catppuccin **token files** off-device (not a live apply).

## Quickshell types actually used

`PanelWindow`, `WlrLayershell` / `WlrLayer.Overlay` / `WlrKeyboardFocus.Exclusive` / `ExclusionMode.Ignore`, `Process` (`stdinEnabled`, `write`, `SplitParser`, `StdioCollector`), `FileView` (`setText`, `atomicWrites`), `IpcHandler`, `Hyprland` is **not** required (hyprctl Process instead), `BarWidget`, `WidgetButton`, `BorderSurface`, `Color.*`, `Style.*`, `Border.*`.

`Quickshell.execDetached` is used only as a summon fallback from the bar chip.

Clipboard is `wl-copy` via `Process`, not an undocumented `Quickshell.clipboard`.

## Helper fallback

Missing `bin/chromad` → `compat/chromad.sh` (grim oneshots + python3 k-means if present). Palette from `source=window` parses `hyprctl -j activewindow` ∩ focused monitor and passes that geometry to grim. `freeze` emits `frame` + `frozen` only (never a `pick`). Frame dirs are removed on EXIT/INT/TERM (shell trap) and on chromad quit/SIGINT/SIGTERM/atexit. Core picker/palette/theme survive with zero Rust binary. OCR/QR hide when `tesseract` / `zbarimg` are absent.

## Review artifacts

`.gpt_review_err.log`, `.gpt_review_r1.md`, `.review_prompt.md` (and similar) stay on disk for the review gate. They are gitignored so they are not packaged. They are **not** deleted.

## Out of scope (intentional)

- Persistent guides (v1.1)
- Millimetre ruler
- A C++ QImage wrapper (plugins cannot reliably register C++ types in the shared shell)
- Writing Hyprland config
- Network, accounts, telemetry
