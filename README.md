# LocalLingo

**LocalLingo** is a free, OS-level voice-to-text tool. Press a global hotkey anywhere on your system, speak, and transcribed text is typed directly into whatever input field currently has focus.

## Features

- **Start instantly** with Groq cloud transcription (free tier), or go fully offline after downloading a local Whisper model
- Global hotkey (default: **Ctrl+Shift+Space**, **Cmd+Shift+Space** on macOS)
- Push-to-talk by default; toggle mode available in settings
- Whisper.cpp ASR locally (`large-v3-turbo` default, auto-tier fallback)
- Silero VAD for speech boundary detection
- Cross-platform: Windows, macOS, Linux (X11 + Wayland)
- Clipboard paste fallback when direct key injection is unavailable
- **Auto-detect OS** and offer to install missing build dependencies on first run

## Tech stack

| Layer | Choice |
|---|---|
| App shell | Tauri 2.x (Rust + minimal React settings UI) |
| ASR (local) | whisper.cpp via `whisper-rs` |
| ASR (cloud fallback) | Groq Whisper API |
| VAD | Silero VAD (ONNX Runtime) |
| Audio | cpal |
| Hotkey | global-hotkey |
| Text injection | Platform-specific (SendInput / CGEvent / uinput / clipboard) |

## Quick start

```bash
npm install
npm start
```

`npm start` runs [`scripts/run.sh`](scripts/run.sh), which:

1. Ensures npm dependencies are installed
2. Detects your OS (Fedora, Debian/Ubuntu, Arch, openSUSE, …)
3. Probes for missing build libraries and prompts **Install now? [y/N]**
4. Offers to install Rust via rustup if missing
5. Launches the app with `npm run tauri dev`

Skip the dependency check:

```bash
LOCAL_LINGO_SKIP_DEPS=1 npm start
```

### Groq cloud transcription (optional)

Use the app **before** downloading a ~550 MB local model:

1. Get a free API key at [console.groq.com](https://console.groq.com)
2. Paste it in **Settings → Cloud fallback (Groq)** or during onboarding
3. Mic test and dictation work immediately via Groq

Once you download a local Whisper model, transcription switches to **fully offline** local inference automatically.

**Privacy:** While no local model is installed, audio is sent to Groq for transcription. The UI shows whether you are on **Local** or **Cloud** mode.

You can also set `GROQ_API_KEY` in your environment instead of saving in Settings.

## Prerequisites

See [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

- Rust (stable) — offered by `npm start` via rustup
- Node.js 18+

### Linux dependency install (manual)

If you decline the interactive prompt, run the script for your distro:

| Distro | Script |
|--------|--------|
| Debian / Ubuntu | `./scripts/setup-linux.sh` |
| Fedora / RHEL | `./scripts/setup-fedora.sh` |
| Arch / Manjaro | `./scripts/setup-arch.sh` |
| openSUSE | `./scripts/setup-opensuse.sh` |

**Fedora example:**

```bash
./scripts/setup-fedora.sh
source "$HOME/.cargo/env"
npm start
```

## Development

```bash
npm install
npm start
# or: ./scripts/dev.sh
```

### Troubleshooting

**`cargo: command not found`**

```bash
source "$HOME/.cargo/env"
# or open a new terminal tab/window
cargo --version
```

**Missing `webkit2gtk-4.1` / `glib-2.0` / build errors on Fedora**

```bash
./scripts/setup-fedora.sh
# or re-run: npm start   and answer Y to the install prompt
```

**`whisper-rs-sys`: `cmake` not installed**

Install build tools via your distro script (see above), then:

```bash
cd src-tauri && cargo clean && cd ..
npm start
```

**Hotkey already in use (X11 BadAccess)**

Change the hotkey in Settings (e.g. `Ctrl+Alt+Space`). GNOME or IBus may reserve `Ctrl+Shift+Space`.

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
| Transcription | Local if model cached, else Groq if API key set |

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

## Network usage

| Mode | Network |
|------|---------|
| Local model cached | No network during dictation |
| Cloud (Groq) | Audio sent to Groq per transcription |
| Model download | HuggingFace (one-time, during onboarding/Settings) |

Build with `--no-default-features --features network-isolation` to disable Groq and model download code paths.

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

## Project structure

```
src-tauri/src/
├── audio/       # cpal capture + Silero VAD
├── asr/         # whisper engine, Groq cloud, model manager, router
├── injection/   # platform text injection
├── hotkey.rs    # global hotkey listener
├── tray.rs      # tray state definitions
├── pipeline.rs  # hotkey → capture → VAD → ASR → inject
└── config.rs    # local TOML settings

scripts/
├── run.sh           # main entry (npm start)
├── setup-deps.sh    # OS detect + Y/N install prompt
├── setup-fedora.sh
├── setup-linux.sh
└── setup-arch.sh
```

## License

MIT (placeholder — set your license before release)
