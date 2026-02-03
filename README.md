███████╗██╗   ██╗███████╗███╗   ███╗ ██████╗ ███╗   ██╗
██╔════╝╚██╗ ██╔╝██╔════╝████╗ ████║██╔═══██╗████╗  ██║
███████╗ ╚████╔╝ ███████╗██╔████╔██║██║   ██║██╔██╗ ██║
╚════██║  ╚██╔╝  ╚════██║██║╚██╔╝██║██║   ██║██║╚██╗██║
███████║   ██║   ███████║██║ ╚═╝ ██║╚██████╔╝██║ ╚████║
╚══════╝   ╚═╝   ╚══════╝╚═╝     ╚═╝ ╚═════╝ ╚═╝  ╚═══╝

Lightweight Windows system monitor for GPU/CPU/RAM/VRAM

## Features

- Below-normal thread priority to avoid interfering with other programs
- Real-time monitoring: GPU, CPU, RAM, VRAM, GPU temperature
- Auto-detects NVIDIA or AMD GPUs, falls back to CPU/RAM only
- Three display modes: Minimal, Graph, Headless
- Automatic CSV logging with timestamps
- Visual warnings when GPU hits 95%+ utilization

## Requirements

- Windows
- NVIDIA or AMD GPU (optional)
- Rust toolchain (for building from source)

## Installation

**Download binary:**

1. Go to [Releases](https://github.com/Gitorian/sysmon/releases)
2. Download the version for your GPU:
   - `sysmon-nvidia.exe` — NVIDIA only
   - `sysmon-amd.exe` — AMD only
   - `sysmon-dual.exe` — both (auto-detects)
3. Run it

**Build from source:**

```bash
git clone https://github.com/Gitorian/sysmon.git
cd sysmon
cargo build --release
```

**Feature flags:**

```bash
# NVIDIA only (default)
cargo build --release --features nvidia

# AMD only
cargo build --release --no-default-features --features amd

# Both
cargo build --release --features "nvidia,amd"
```

## Usage

Run the executable and select a display mode:

**Minimal** - Compact text display

```
┌─ Current ──────────────────────────────────────────────────────┐
│ GPU: 87% │ CPU: 45% │ VRAM:6543MB (68%) │ Temp:72°C │ RAM: 54% │
├─ Maximum ──────────────────────────────────────────────────────┤
│ GPU: 95% │ CPU: 78% │ VRAM:7890MB (82%) │ Temp:75°C │ RAM: 67% │
└────────────────────────────────────────────────────────────────┘
```

**Graph** - Full-screen ASCII graph with 300 data points of history

**Headless** - Background logging only, no UI overhead

All modes create timestamped CSV logs in the executable directory.

## Configuration

**Sampling intervals:**

- Interactive modes: 1s, 2s, 5s
- Headless mode: 1s, 2s, 5s, 10s, 30s

## Technical Details

**Architecture:**

- CPU/RAM: Windows API (`GetSystemTimes`, `GlobalMemoryStatusEx`)
- GPU: NVML (NVIDIA) or ADLX (AMD) with automatic detection
- Terminal: `crossterm` with raw mode
- Logging: 16KB buffered writes
- Modular design: separate modules for metrics, GPU backends, display modes

**Optimizations:**

- Hot path inlining
- Circular buffers (no reallocation)
- Minimal dependencies with `default-features = false`
- Non-blocking I/O

## License

MIT License - See [LICENSE](LICENSE)

## Dependencies

- [nvml-wrapper](https://github.com/Cldfire/nvml-wrapper)
- [adlx](https://crates.io/crates/adlx)
- [crossterm](https://github.com/crossterm-rs/crossterm)
- [chrono](https://github.com/chronotope/chrono)
- [anyhow](https://github.com/dtolnay/anyhow)
- Windows API
