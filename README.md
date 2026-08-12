# LocalLingo

**LocalLingo** is a free, fully on-device, OS-level voice-to-text tool. Press a global hotkey anywhere on your system, speak, and transcribed text is typed directly into whatever input field currently has focus.

## Features

- 100% offline after initial model download — no cloud API calls, no telemetry
- Global hotkey (default: **Ctrl+Shift+Space**, **Cmd+Shift+Space** on macOS)
- Push-to-talk by default; toggle mode available in settings
- Whisper.cpp ASR (`large-v3-turbo` default, auto-tier fallback)
- Silero VAD for speech boundary detection
- Cross-platform: Windows, macOS, Linux (X11 + Wayland)
- Clipboard paste fallback when direct key injection is unavailable

## Tech stack

| Layer | Choice |
|---|---|
| App shell | Tauri 2.x (Rust + minimal React settings UI) |
| ASR | whisper.cpp via `whisper-rs` |
| VAD | Silero VAD (ONNX Runtime) |
| Audio | cpal |
| Hotkey | global-hotkey |
| Text injection | Platform-specific (SendInput / CGEvent / uinput / clipboard) |

## Prerequisites

See [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

- Rust (stable)
- Node.js 18+
- Linux: `libwebkit2gtk-4.1-dev`, `libasound2-dev`, `libx11-dev`, `libxkbcommon-dev`

## Development

```bash
npm install

# Linux: install system libraries once (requires sudo)
chmod +x scripts/setup-linux.sh && ./scripts/setup-linux.sh

# Ensure Rust is on PATH (new terminals pick this up from ~/.bashrc)
source "$HOME/.cargo/env"

npm run tauri dev
# or: ./scripts/dev.sh
```

### Troubleshooting

**`cargo: command not found`**

Rust is installed via rustup to `~/.cargo/bin`. Your current terminal may have started before that was added to PATH. Fix:

```bash
source "$HOME/.cargo/env"
# or open a new terminal tab/window
cargo --version
```

**`failed to run cargo metadata` / Tauri can't find cargo**

Same fix — Tauri invokes `cargo` from PATH. Run `source "$HOME/.cargo/env"` first, or use `./scripts/dev.sh`.

**`Package 'glib-2.0' / 'webkit2gtk-4.1' / 'pango' was not found`**

Install Tauri Linux dependencies:

```bash
./scripts/setup-linux.sh
```

**`whisper-rs-sys`: `cmake` not installed / `stdbool.h` file not found**

whisper.cpp is compiled via CMake and needs a C/C++ toolchain + clang for bindgen:

```bash
sudo apt-get install -y cmake clang libclang-dev build-essential
# or re-run the full setup script:
./scripts/setup-linux.sh
```

Then clean and rebuild:

```bash
cd src-tauri && cargo clean && cd ..
npm run tauri dev
```

### Debug audio recording (CLI, no UI)

```bash
source "$HOME/.cargo/env"
cd src-tauri
cargo run -- --debug-record 5
# Saves WAV to ~/.local/share/local-lingo/cli-debug.wav
```

## Default decisions

| Setting | Default |
|---|---|
| Hotkey mode | Push-to-talk |
| Hotkey | Ctrl+Shift+Space |
| Trailing silence | 800 ms |
| Default model tier | High (`large-v3-turbo-q5_0`) |

## Permissions

| OS | Required |
|---|---|
| Windows | Microphone |
| macOS | Microphone + Accessibility (Privacy & Security) |
| Linux X11 | Display server access |
| Linux Wayland | `input` group + `/dev/uinput` udev rule; clipboard fallback always available |

### Linux Wayland udev rule

```bash
# /etc/udev/rules.d/99-uinput.rules
KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"
```

Then: `sudo usermod -aG input $USER` and reboot.

## Network isolation

After models are cached, LocalLingo makes **zero network requests** during dictation. HTTP is only used in `asr/model_manager.rs` for model download (gated by the `model-download` feature).

Verify with:

```bash
# Block network and run a dictation session — should still work
sudo iptables -A OUTPUT -p tcp --dport 443 -j DROP
# ... use LocalLingo ...
sudo iptables -D OUTPUT -p tcp --dport 443 -j DROP
```

Build with `--no-default-features --features network-isolation` to disable download code paths in release builds that ship pre-cached models.

## WER benchmark

```bash
chmod +x scripts/benchmark_wer.sh
./scripts/benchmark_wer.sh
```

Add WAV + reference `.txt` pairs under `test-data/dev-jargon/`.

## Building release

```bash
npm run tauri build
```

Platform-specific GPU acceleration for whisper:

```bash
# macOS
cargo build --features whisper-rs/metal

# Linux NVIDIA
cargo build --features whisper-rs/cuda

# Linux fallback
cargo build --features whisper-rs/vulkan
```

## Project structure

```
src-tauri/src/
├── audio/       # cpal capture + Silero VAD
├── asr/         # whisper engine, model manager, postprocess
├── injection/   # platform text injection
├── hotkey.rs    # global hotkey listener
├── tray.rs      # tray state definitions
├── pipeline.rs  # hotkey → capture → VAD → ASR → inject
└── config.rs    # local TOML settings
```

## License

MIT (placeholder — set your license before release)
