# Zed-Style System Monitor (GPUI)

A fast, hardware-accelerated Linux system monitor built in Rust using `gpui-kit` and `sysinfo`, designed with a crisp Zed Workspace aesthetic.

## Screenshots

### Dashboard (Sidebar Split View)
<!-- Paste your Dashboard screenshot right here on GitHub -->
<img width="400" alt="image" src="https://github.com/user-attachments/assets/697148ad-3c66-4014-a1a8-3e1ae8761563" />
<img width="400" alt="image" src="https://github.com/user-attachments/assets/7cb41fe3-ad4f-4689-b871-8c357316f5c2" />


### Settings, Memory Strategy & Themes
<!-- Paste your Settings screenshot right here on GitHub -->
<img width="300" alt="image" src="https://github.com/user-attachments/assets/a1a38f14-6368-432c-bca4-08ea36594e5f" />


## Features

- **GPU-Accelerated GPUI Interface:** Idle-aware 60fps delta-time animations with near-zero idle CPU usage.
- **iOS Bouncy Rubber-Band Scrolling:** Custom spring physics and kinetic momentum scrolling on the process table, sidebar, and settings view.
- **Dual Memory Accounting Modes:**
  - **Fast RSS (`statm`):** Instantaneous, zero-overhead Resident Set Size lookup (`2000ms` default).
  - **True PSS (`smaps_rollup`):** Reads the Linux kernel's exact Proportional Set Size (PSS), dividing shared libraries proportionally across processes for accurate per-app RAM totals.
- **Arch Linux `zram` Awareness:** Automatically detects `/sys/block/zram*` in-RAM compressed swap, active compression algorithms (`zstd` / `lzo`), and live compression ratios alongside nominal hardware RAM (`24G HW`).
- **Multi-GPU Telemetry:** Simultaneous detection and monitoring for both Integrated GPUs (`/sys/class/drm/card*` Intel iGPU / AMD Radeon) and Discrete GPUs (`nvidia-smi` NVIDIA GeForce RTX), including engine load, clock speed (`MHz`), temperature, and VRAM.
- **Log-Scaled Network & Live Sparklines:** Real-time RX/TX bandwidth meters using logarithmic scaling paired with 32-bar live waveform sparklines.
- **Live Layout Customizer:** Switch between **Sidebar Split** (default), **2x2 Quad Grid**, and **4-Card Top Ribbon** layouts on the fly.
- **Instant Process Search, Grouping & Signal Inspector:** Press `/` to filter processes by name or PID, filter by `>5% CPU` or `>100M RAM`, toggle **Grouped** vs. **PIDs**, and click any process row to send `TERM` or `KILL`.
- **5 Built-in Color Themes:** *Zed Ayu Mirage*, *Zed One Dark*, *Catppuccin Mocha*, *GTK Original Purple*, and *Zed One Light*.

## Keyboard Shortcuts

| Key | Action |
| :--- | :--- |
| `/` | Focus the Process Search bar |
| `Esc` | Clear search filter & unfocus search |
| `Space` | Pause / Resume live telemetry polling |
| `Click Row` | Inspect process PSS/RSS/Virtual memory & reveal `TERM` / `KILL` actions |

## Build & Run

~~~bash
cargo run --release
~~~

## Project Structure

~~~text
src/
├── main.rs          # Application entry point & window initialization
├── state.rs         # Background telemetry collector (/proc, sysfs, DRM, zram, nvidia-smi)
├── theme.rs         # Zed & Catppuccin color palettes
└── ui/
    ├── mod.rs       # Root view state, 60fps delta-time loop & keyboard handler
    ├── scroller.rs  # iOS spring physics & rubber-band overscroll engine
    ├── header.rs    # Top workspace bar & live Layout Customizer drawer
    ├── telemetry.rs # CPU, RAM/zram, Multi-GPU & Network sparkline cards
    ├── processes.rs # Search bar, quick filters, sortable table & kill inspector
    └── settings.rs  # Refresh frequency, RSS/PSS strategy & theme selector
~~~
