# sysmon - Lightweight System Monitor

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![Language](https://img.shields.io/badge/language-Rust-orange)
![License](https://img.shields.io/badge/license-MIT-green)

A **minimal-impact** system monitoring tool designed for gamers and performance enthusiasts. Tracks GPU, CPU, RAM, and VRAM usage while running at **below-normal thread priority** to minimize input lag and latency for other programs.

## ✨ Features

- **🎮 Gaming-Optimized**: Runs at below-normal thread priority to avoid interfering with games
- **📊 Real-Time Monitoring**: GPU utilization, CPU usage, RAM usage, VRAM usage, GPU temperature
- **📈 Three Display Modes**:
  - **Minimal**: Compact text-based display
  - **Graph**: Full-screen ASCII graph with historical data (300 data points)
  - **Headless**: Background logging only (perfect for benchmarking)
- **📝 CSV Logging**: Automatic timestamped logs for analysis
- **⚠️ Performance Alerts**: Visual warnings when GPU hits 95%+ utilization
- **⚡ Extremely Lightweight**: 
  - Optimized release build with LTO and size optimization
  - Minimal memory footprint
  - Non-blocking event handling

## 🎯 Why sysmon?

Most monitoring tools run at normal or high priority, which can cause micro-stutters and increased input latency during gaming. **sysmon** is specifically designed to:

- Run at **below-normal thread priority** by default
- Use minimal CPU cycles (typically <0.1% CPU usage)
- Avoid aggressive polling that causes system interference
- Provide accurate metrics without impacting game performance

## 📋 Requirements

- **Windows** (uses Windows API for CPU/RAM metrics)
- **NVIDIA GPU** (uses NVML for GPU metrics)
- **Rust toolchain** (for building from source)

## 🚀 Installation

### Option 1: Download Release Binary

1. Go to [Releases](https://github.com/Gitorian/sysmon/releases)
2. Download `sysmon.exe`
3. Run directly - no installation needed!

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/Gitorian/sysmon.git
cd sysmon

# Build optimized release binary
cargo build --release

# Run
target\release\sysmon.exe
```

## 🎮 Usage

### Quick Start

Simply run `sysmon.exe` and follow the interactive menu:

```
System Monitor
========================================

Select display mode:
1 = Minimal
2 = Graph
3 = Headless (background logging)

Choice (1/2/3): _
```

### Display Modes

#### 1️⃣ Minimal Mode
Compact real-time display with current and peak values:

```
System Monitor
Log: sysmon_2026-02-03_14-30-00.csv
Interval: 1s | Priority: Below Normal

Now: GPU: 87% | CPU: 45% | VRAM:6543(68%) | T:72°C | RAM:54%
Max: GPU: 95% | CPU: 78% | VRAM:7890(82%) | T:75°C | RAM:67%

Ctrl+C to quit
```

#### 2️⃣ Graph Mode
Full-screen ASCII graph with historical data:

```
100% ┤
 75% │        ███
 50% │    ███████▓▓▓
 25% │▒▒▒▒███████▓▓▓▓▓
  0% └────────────────────
     █GPU ▓CPU ▒RAM

Now: GPU: 87%|CPU: 45%|VRAM:6543(68%)|T:72°C|RAM:54%
Max: GPU: 95%|CPU: 78%|VRAM:7890(82%)|T:75°C|RAM:67%|Pts:150
```

#### 3️⃣ Headless Mode
Background logging without UI overhead - perfect for long benchmarks:

```
Running in headless mode...
Logging every 5s to: sysmon_2026-02-03_14-30-00.csv
Press Ctrl+C to stop

[2026-02-03 14:30:05] GPU:87% CPU:45% RAM:54% T:72°C
```

### Logging

All modes automatically create timestamped CSV logs in the same directory as the executable:

```csv
Timestamp,GPU%,CPU%,RAM%,GPU_Temp,GPU_Mem_MiB,GPU_Mem_%
2026-02-03 14:30:00,87,45,54,72,6543,68
2026-02-03 14:30:01,89,48,55,73,6589,69
```

Perfect for:
- Benchmark analysis
- Performance tracking over time
- Identifying bottlenecks
- Thermal analysis

## 🔧 Configuration

### Sampling Intervals

- **Interactive modes** (Minimal/Graph): 1s, 2s, or 5s
- **Headless mode**: 1s, 2s, 5s, 10s, or 30s

### Thread Priority

Automatically set to **Below Normal** to minimize impact on other applications. You can verify this in the status line:

```
✓ Priority: Below Normal (-1)
```

## 🛠️ Technical Details

### Architecture

- **CPU Metrics**: Windows `GetSystemTimes` API (native, zero-copy)
- **RAM Metrics**: Windows `GlobalMemoryStatusEx` API
- **GPU Metrics**: NVIDIA Management Library (NVML)
- **Terminal UI**: `crossterm` with raw mode for responsive input
- **Logging**: Buffered writes (16KB buffer) to minimize I/O overhead

### Optimizations

```toml
[profile.release]
opt-level = "z"          # Optimize for size
lto = true               # Link-time optimization
codegen-units = 1        # Maximum optimization
strip = true             # Strip debug symbols
panic = "abort"          # Smaller binary
overflow-checks = false  # Performance boost
```

- **Hot path inlining**: Critical functions marked with `#[inline(always)]`
- **Circular buffers**: History tracking without reallocation
- **Minimal dependencies**: Only essential crates with `default-features = false`
- **Non-blocking I/O**: Event polling prevents thread blocking

### Performance Impact

- **CPU Usage**: <0.1% on modern CPUs
- **Memory**: ~2-5 MB
- **Thread Priority**: Below Normal (-1)
- **I/O**: Buffered writes every N seconds (configurable)

## 📊 Use Cases

### Gaming
Run in **Minimal** or **Headless** mode while gaming to:
- Monitor GPU/CPU bottlenecks without impacting performance
- Track thermals during extended sessions
- Verify your system isn't throttling

### Benchmarking
Use **Headless** mode for clean benchmark runs:
- Zero UI overhead
- Timestamped CSV logs for analysis
- Long-term stability testing

### Development
Use **Graph** mode while developing:
- Visual feedback on performance changes
- Real-time bottleneck identification
- Historical data for before/after comparisons

## 🤝 Contributing

Contributions welcome! Areas of interest:

- [ ] AMD GPU support (via ADL/ROCm)
- [ ] Linux support (via sysfs/procfs)
- [ ] Custom alert thresholds
- [ ] Export to other formats (JSON, Prometheus)
- [ ] Multi-GPU support
- [ ] Network monitoring

## 📄 License

MIT License - See [LICENSE](LICENSE) file for details

## 🙏 Acknowledgments

- [nvml-wrapper](https://github.com/Cldfire/nvml-wrapper) - NVIDIA GPU monitoring
- [crossterm](https://github.com/crossterm-rs/crossterm) - Cross-platform terminal manipulation
- Windows API - Native system metrics

## 📞 Support

Found a bug? Have a feature request? [Open an issue](https://github.com/Gitorian/sysmon/issues)!

---

**Made with 🦀 Rust for maximum performance and minimal overhead**