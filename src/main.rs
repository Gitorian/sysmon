//! System Monitor - Lightweight GPU/CPU/RAM/VRAM monitoring tool
//! Optimized for minimal gaming impact with below-normal thread priority

use std::io::{self, Write, BufWriter};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    cursor, execute,
    style::{Color, ResetColor, SetForegroundColor, SetBackgroundColor},
    terminal::{self, Clear, ClearType}
};
use windows::Win32::System::{
    SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
    Threading::{GetSystemTimes, SetThreadPriority, GetCurrentThread, GetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL},
};
use windows::Win32::Foundation::FILETIME;
use nvml_wrapper::Nvml;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// System metrics snapshot
#[derive(Clone, Copy, Default)]
struct Metrics {
    cpu: u32,           // CPU usage percentage (0-100)
    ram: u32,           // RAM usage percentage (0-100)
    gpu: u32,           // GPU usage percentage (0-100)
    gpu_temp: u32,      // GPU temperature in Celsius
    gpu_mem: u32,       // GPU memory used in MiB
    gpu_mem_total: u32, // GPU memory total in MiB
}

/// Display mode selection
enum DisplayMode {
    Minimal,   // Compact text display
    Graph,     // Full-screen graph with history
    Headless,  // Background logging only
}

/// CPU usage tracker using Windows GetSystemTimes API
struct CpuTracker {
    prev_idle: u64,   // Previous idle time
    prev_kernel: u64, // Previous kernel time
    prev_user: u64,   // Previous user time
}

impl CpuTracker {
    /// Create new CPU tracker
    #[inline]
    fn new() -> Self {
        Self { prev_idle: 0, prev_kernel: 0, prev_user: 0 }
    }

    /// Calculate CPU usage percentage based on time deltas
    /// Returns 0-100 representing CPU usage
    #[inline]
    fn get_cpu_usage(&mut self) -> u32 {
        unsafe {
            let mut idle = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            if GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).is_ok() {
                // Convert FILETIME to u64 (100-nanosecond intervals since 1601-01-01)
                let idle_time = ((idle.dwHighDateTime as u64) << 32) | (idle.dwLowDateTime as u64);
                let kernel_time = ((kernel.dwHighDateTime as u64) << 32) | (kernel.dwLowDateTime as u64);
                let user_time = ((user.dwHighDateTime as u64) << 32) | (user.dwLowDateTime as u64);
                
                // Calculate usage only if we have previous samples
                if self.prev_idle > 0 {
                    let idle_diff = idle_time.saturating_sub(self.prev_idle);
                    let total_diff = (kernel_time + user_time).saturating_sub(self.prev_kernel + self.prev_user);
                    if total_diff > 0 {
                        // CPU% = (total_time - idle_time) / total_time * 100
                        let cpu = ((total_diff - idle_diff) * 100) / total_diff;
                        self.prev_idle = idle_time;
                        self.prev_kernel = kernel_time;
                        self.prev_user = user_time;
                        return cpu.min(100) as u32;
                    }
                }
                
                // Store current values for next calculation
                self.prev_idle = idle_time;
                self.prev_kernel = kernel_time;
                self.prev_user = user_time;
            }
        }
        0
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Generate timestamp for CSV logging (YYYY-MM-DD HH:MM:SS)
#[inline]
fn timestamp() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let secs = now % 86400;
    let days = now / 86400;
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        1970 + (days + 365) / 365,
        ((days % 365) / 30).min(11) + 1,
        (days % 365 % 30) + 1,
        secs / 3600 % 24,
        secs / 60 % 60,
        secs % 60)
}

/// Generate timestamp for filename (YYYY-MM-DD_HH-MM-SS)
#[inline]
fn timestamp_filename() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let secs = now % 86400;
    let days = now / 86400;
    format!("{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
        1970 + (days + 365) / 365,
        ((days % 365) / 30).min(11) + 1,
        (days % 365 % 30) + 1,
        secs / 3600 % 24,
        secs / 60 % 60,
        secs % 60)
}

/// Convert Windows thread priority value to human-readable name
fn get_priority_name(priority: i32) -> &'static str {
    match priority {
        -2 => "Idle",
        -1 => "Below Normal",
        0 => "Normal",
        1 => "Above Normal",
        2 => "Highest",
        15 => "Time Critical",
        _ => "Unknown"
    }
}

/// Print colored text to stdout
#[inline]
fn print_colored(text: &str, color: Color) {
    execute!(io::stdout(), SetForegroundColor(color)).ok();
    print!("{}", text);
    execute!(io::stdout(), ResetColor).ok();
}

/// Print menu item with color
#[inline]
fn print_menu(text: &str, color: Color) {
    execute!(io::stdout(), SetForegroundColor(color)).ok();
    println!("{}", text);
    execute!(io::stdout(), ResetColor).ok();
}

// ============================================================================
// METRICS COLLECTION
// ============================================================================

/// Collect all system metrics in one pass
/// Optimized with inline(always) for hot path performance
#[inline(always)]
fn collect_metrics(cpu_tracker: &mut CpuTracker, nvml: &Option<Nvml>) -> Metrics {
    // CPU usage via Windows API
    let cpu = cpu_tracker.get_cpu_usage();
    
    // RAM usage via GlobalMemoryStatusEx
    let ram = unsafe {
        let mut mem = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        let _ = GlobalMemoryStatusEx(&mut mem);
        mem.dwMemoryLoad
    };
    
    // GPU metrics via NVML
    let (gpu, gpu_temp, gpu_mem, gpu_mem_total) = nvml.as_ref()
        .and_then(|n| n.device_by_index(0).ok())
        .map(|d| {
            let util = d.utilization_rates().map(|u| u.gpu as u32).unwrap_or(0);
            let temp = d.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                .unwrap_or(0) as u32;
            let (mem, total) = d.memory_info()
                .map(|m| ((m.used >> 20) as u32, (m.total >> 20) as u32)) // Convert bytes to MiB
                .unwrap_or((0, 0));
            (util, temp, mem, total)
        })
        .unwrap_or((0, 0, 0, 0));
    
    Metrics { cpu, ram, gpu, gpu_temp, gpu_mem, gpu_mem_total }
}

// ============================================================================
// RENDERING FUNCTIONS
// ============================================================================

/// Render minimal display mode
fn render_minimal(
    stdout: &mut io::Stdout,
    m: &Metrics,
    max: &Metrics,
    warning: bool,
    gpu95: bool,
    log_filename: &str,
    interval: u64,
    priority: &str
) -> io::Result<()> {
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    print_colored("System Monitor\n", Color::Cyan);
    print_colored(&format!("Log: {}\n", log_filename), Color::Green);
    print_colored(&format!("Interval: {}s | Priority: {}\n\n", interval, priority), Color::DarkGrey);
    
    // Current metrics
    print!("Now: ");
    if warning {
        execute!(stdout, SetBackgroundColor(Color::Red), SetForegroundColor(Color::White))?;
        print!("GPU:{:3}%", m.gpu);
        execute!(stdout, ResetColor)?;
    } else {
        print_colored(&format!("GPU:{:3}%", m.gpu), Color::Green);
    }
    
    print!(" | ");
    print_colored(&format!("CPU:{:3}%", m.cpu), Color::Blue);
    print!(" | ");
    let vram_pct = if m.gpu_mem_total > 0 { (m.gpu_mem * 100) / m.gpu_mem_total } else { 0 };
    print_colored(&format!("VRAM:{}({}%)", m.gpu_mem, vram_pct), Color::Cyan);
    print!(" | ");
    print_colored(&format!("T:{}°C", m.gpu_temp), Color::Red);
    print!(" | ");
    print_colored(&format!("RAM:{}%\n", m.ram), Color::Magenta);
    
    // Maximum metrics
    print!("Max: ");
    if gpu95 && max.gpu >= 95 {
        execute!(stdout, SetBackgroundColor(Color::Red), SetForegroundColor(Color::White))?;
        print!("GPU:{:3}%", max.gpu);
        execute!(stdout, ResetColor)?;
    } else {
        print_colored(&format!("GPU:{:3}%", max.gpu), Color::Green);
    }
    
    print!(" | ");
    print_colored(&format!("CPU:{:3}%", max.cpu), Color::Blue);
    print!(" | ");
    let max_vram_pct = if max.gpu_mem_total > 0 { (max.gpu_mem * 100) / max.gpu_mem_total } else { 0 };
    print_colored(&format!("VRAM:{}({}%)", max.gpu_mem, max_vram_pct), Color::Cyan);
    print!(" | ");
    print_colored(&format!("T:{}°C", max.gpu_temp), Color::Red);
    print!(" | ");
    print_colored(&format!("RAM:{}%\n", max.ram), Color::Magenta);
    
    // Warning if GPU hit 95%+
    if gpu95 {
        println!();
        execute!(stdout, SetForegroundColor(Color::Yellow), SetBackgroundColor(Color::Red))?;
        println!("! GPU ≥95%");
        execute!(stdout, ResetColor)?;
    }
    
    println!("\nCtrl+C to quit");
    stdout.flush()
}

/// Render graph display mode with history
fn render_graph(
    stdout: &mut io::Stdout,
    m: &Metrics,
    max: &Metrics,
    gpu95: bool,
    history: &[Metrics],
    log_filename: &str,
    interval: u64,
    priority: &str
) -> io::Result<()> {
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    print_colored("System Monitor\n", Color::Cyan);
    print_colored(&format!("Log: {}\n", log_filename), Color::Green);
    print_colored(&format!("Interval: {}s | Priority: {}\n\n", interval, priority), Color::DarkGrey);
    
    // Calculate graph dimensions based on terminal size
    let (tw, th) = terminal::size()?;
    let gw = (tw as usize).saturating_sub(10).max(40);  // Graph width
    let gh = (th as usize).saturating_sub(16).max(15);  // Graph height
    
    // Auto-scale Y axis based on max value in history
    let max_val = history.iter()
        .map(|m| m.gpu.max(m.cpu).max(m.ram))
        .max()
        .unwrap_or(100);
    let yscale = if max_val <= 50 { 50 } else if max_val <= 75 { 75 } else { 100 };
    
    // Draw graph
    println!("100% ┤");
    for y in (0..gh).rev() {
        print!("{:3}% │", (y * yscale as usize) / gh);
        
        // Calculate sampling step for X axis (downsample if needed)
        let step = if history.len() <= gw { 1 } else { (history.len() * 10) / gw };
        
        for x in 0..gw.min(history.len()) {
            let idx = if step == 1 { x } else { (x * step) / 10 };
            let s = history[idx.min(history.len() - 1)];
            
            // Scale metrics to graph height
            let gy = ((s.gpu as usize * gh) / yscale as usize).min(gh);
            let cy = ((s.cpu as usize * gh) / yscale as usize).min(gh);
            let ry = ((s.ram as usize * gh) / yscale as usize).min(gh);
            
            // Priority rendering: GPU > CPU > RAM
            if gy >= y {
                print_colored("█", Color::Green);
            } else if cy >= y {
                print_colored("▓", Color::Blue);
            } else if ry >= y {
                print_colored("▒", Color::Magenta);
            } else {
                print!(" ");
            }
        }
        println!();
    }
    
    // Draw X axis
    print!("  0% └");
    for _ in 0..gw { print!("─"); }
    println!();
    
    // Legend
    print!("     ");
    print_colored("█GPU ", Color::Green);
    print_colored("▓CPU ", Color::Blue);
    print_colored("▒RAM", Color::Magenta);
    if gpu95 {
        print!(" ");
        execute!(stdout, SetForegroundColor(Color::Yellow), SetBackgroundColor(Color::Red))?;
        print!("!GPU≥95%");
        execute!(stdout, ResetColor)?;
    }
    println!("\n");
    
    // Current metrics
    print!("Now: ");
    if m.gpu >= 95 {
        execute!(stdout, SetBackgroundColor(Color::Red), SetForegroundColor(Color::White))?;
        print!("GPU:{:3}%", m.gpu);
        execute!(stdout, ResetColor)?;
    } else {
        print_colored(&format!("GPU:{:3}%", m.gpu), Color::Green);
    }
    
    print!("|");
    print_colored(&format!("CPU:{:3}%", m.cpu), Color::Blue);
    print!("|");
    let vp = if m.gpu_mem_total > 0 { (m.gpu_mem * 100) / m.gpu_mem_total } else { 0 };
    print_colored(&format!("VRAM:{}({}%)", m.gpu_mem, vp), Color::Cyan);
    print!("|");
    print_colored(&format!("T:{}°C", m.gpu_temp), Color::Red);
    print!("|");
    print_colored(&format!("RAM:{}%\n", m.ram), Color::Magenta);
    
    // Maximum metrics
    print!("Max: ");
    if gpu95 && max.gpu >= 95 {
        execute!(stdout, SetBackgroundColor(Color::Red), SetForegroundColor(Color::White))?;
        print!("GPU:{:3}%", max.gpu);
        execute!(stdout, ResetColor)?;
    } else {
        print_colored(&format!("GPU:{:3}%", max.gpu), Color::Green);
    }
    
    print!("|");
    print_colored(&format!("CPU:{:3}%", max.cpu), Color::Blue);
    print!("|");
    let mvp = if max.gpu_mem_total > 0 { (max.gpu_mem * 100) / max.gpu_mem_total } else { 0 };
    print_colored(&format!("VRAM:{}({}%)", max.gpu_mem, mvp), Color::Cyan);
    print!("|");
    print_colored(&format!("T:{}°C", max.gpu_temp), Color::Red);
    print!("|");
    print_colored(&format!("RAM:{}%", max.ram), Color::Magenta);
    println!("|Pts:{}", history.len());
    
    println!("\nCtrl+C to quit");
    stdout.flush()
}

// ============================================================================
// MAIN FUNCTION
// ============================================================================

fn main() -> io::Result<()> {
    // Set thread priority to below normal for minimal gaming impact
    unsafe { let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL); }
    let priority = unsafe { GetThreadPriority(GetCurrentThread()) };
    let priority_name = get_priority_name(priority);
    
    // Setup log file with timestamp
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let log_filename = format!("sysmon_{}.csv", timestamp_filename());
    let log_path = exe_dir.join(&log_filename);
    let log_file = OpenOptions::new().create(true).append(true).open(&log_path)?;
    let mut log_writer = BufWriter::with_capacity(16384, log_file); // 16KB buffer for efficiency
    
    // Write CSV header
    writeln!(log_writer, "Timestamp,GPU%,CPU%,RAM%,GPU_Temp,GPU_Mem_MiB,GPU_Mem_%")?;
    
    // Display mode selection menu
    print_menu("System Monitor", Color::Cyan);
    println!("========================================\n");
    print_menu("Select display mode:", Color::White);
    print_menu("1 = Minimal", Color::Green);
    print_menu("2 = Graph", Color::Yellow);
    print_menu("3 = Headless (background logging)\n", Color::DarkGrey);
    print!("Choice (1/2/3): ");
    io::stdout().flush()?;
    
    let mut input = String::with_capacity(16);
    io::stdin().read_line(&mut input)?;
    let mode = match input.trim() {
        "2" => DisplayMode::Graph,
        "3" => DisplayMode::Headless,
        _ => DisplayMode::Minimal,
    };
    
    // Interval selection (headless mode has extended options)
    let interval = if matches!(mode, DisplayMode::Headless) {
        println!();
        print_menu("Select interval:", Color::White);
        print_menu("1 = 1 second", Color::Yellow);
        print_menu("2 = 2 seconds", Color::Green);
        print_menu("3 = 5 seconds", Color::Cyan);
        print_menu("4 = 10 seconds", Color::Magenta);
        print_menu("5 = 30 seconds\n", Color::DarkGrey);
        print!("Choice (1/2/3/4/5): ");
        io::stdout().flush()?;
        input.clear();
        io::stdin().read_line(&mut input)?;
        match input.trim() {
            "2" => 2,
            "3" => 5,
            "4" => 10,
            "5" => 30,
            _ => 1,
        }
    } else {
        println!();
        print_menu("Select interval:", Color::White);
        print_menu("1 = 1 second", Color::Yellow);
        print_menu("2 = 2 seconds", Color::Green);
        print_menu("3 = 5 seconds\n", Color::Cyan);
        print!("Choice (1/2/3): ");
        io::stdout().flush()?;
        input.clear();
        io::stdin().read_line(&mut input)?;
        match input.trim() {
            "2" => 2,
            "3" => 5,
            _ => 1,
        }
    };
    
    // Initialize NVML for GPU monitoring
    let nvml = Nvml::init().ok();
    if nvml.is_some() { println!("\n✓ GPU monitoring available"); }
    println!("✓ Priority: {} ({})", priority_name, priority);
    println!("✓ Log: {}", log_filename);
    if matches!(mode, DisplayMode::Headless) {
        println!("\nRunning in headless mode...");
        println!("Logging every {}s to: {}", interval, log_filename);
        println!("Press Ctrl+C to stop\n");
    } else {
        println!("\nStarting... Press Ctrl+C to quit\n");
        std::thread::sleep(Duration::from_millis(500));
    }
    
    // Initialize CPU tracker and prime it
    let mut cpu_tracker = CpuTracker::new();
    cpu_tracker.get_cpu_usage();
    std::thread::sleep(Duration::from_millis(100));
    
    // Setup Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::Relaxed))
        .expect("Error setting Ctrl+C handler");
    
    // Tracking variables
    let mut max = Metrics::default();
    let mut gpu95_hit = false;
    let mut log_counter = 0u64;
    let mut print_counter = 0u64;
    
    // HEADLESS MODE - No UI overhead
    if matches!(mode, DisplayMode::Headless) {
        while running.load(Ordering::Relaxed) {
            let m = collect_metrics(&mut cpu_tracker, &nvml);
            
            // Update maximums
            max.cpu = max.cpu.max(m.cpu);
            max.ram = max.ram.max(m.ram);
            max.gpu = max.gpu.max(m.gpu);
            max.gpu_temp = max.gpu_temp.max(m.gpu_temp);
            max.gpu_mem = max.gpu_mem.max(m.gpu_mem);
            if m.gpu_mem_total > 0 { max.gpu_mem_total = m.gpu_mem_total; }
            if m.gpu >= 95 { gpu95_hit = true; }
            
            // Log to CSV
            log_counter += 1;
            if log_counter >= interval {
                let vram_pct = if m.gpu_mem_total > 0 { (m.gpu_mem * 100) / m.gpu_mem_total } else { 0 };
                writeln!(log_writer, "{},{},{},{},{},{},{}",
                    timestamp(), m.gpu, m.cpu, m.ram, m.gpu_temp, m.gpu_mem, vram_pct)?;
                log_writer.flush()?;
                log_counter = 0;
                
                // Print status update every 10 log entries
                print_counter += 1;
                if print_counter >= 10 {
                    println!("[{}] GPU:{}% CPU:{}% RAM:{}% T:{}°C",
                        timestamp(), m.gpu, m.cpu, m.ram, m.gpu_temp);
                    print_counter = 0;
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    // UI MODES - Minimal or Graph
    else {
        let mut stdout = io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
        
        // Allocate history buffer for graph mode (circular buffer)
        let max_history = if matches!(mode, DisplayMode::Graph) { 300 } else { 0 };
        let mut history = Vec::with_capacity(max_history);
        let mut history_pos = 0usize;
        
        while running.load(Ordering::Relaxed) {
            let m = collect_metrics(&mut cpu_tracker, &nvml);
            
            // Update maximums
            max.cpu = max.cpu.max(m.cpu);
            max.ram = max.ram.max(m.ram);
            max.gpu = max.gpu.max(m.gpu);
            max.gpu_temp = max.gpu_temp.max(m.gpu_temp);
            max.gpu_mem = max.gpu_mem.max(m.gpu_mem);
            if m.gpu_mem_total > 0 { max.gpu_mem_total = m.gpu_mem_total; }
            if m.gpu >= 95 { gpu95_hit = true; }
            
            // Update history (circular buffer to avoid reallocation)
            if max_history > 0 {
                if history.len() < max_history {
                    history.push(m);
                } else {
                    history[history_pos] = m;
                    history_pos = (history_pos + 1) % max_history;
                }
            }
            
            // Render UI
            match mode {
                DisplayMode::Minimal => render_minimal(&mut stdout, &m, &max, m.gpu >= 95,
                    gpu95_hit, &log_filename, interval, priority_name)?,
                DisplayMode::Graph => render_graph(&mut stdout, &m, &max, gpu95_hit,
                    &history, &log_filename, interval, priority_name)?,
                DisplayMode::Headless => unreachable!(),
            }
            
            // Log to CSV
            log_counter += 1;
            if log_counter >= interval {
                let vram_pct = if m.gpu_mem_total > 0 { (m.gpu_mem * 100) / m.gpu_mem_total } else { 0 };
                writeln!(log_writer, "{},{},{},{},{},{},{}",
                    timestamp(), m.gpu, m.cpu, m.ram, m.gpu_temp, m.gpu_mem, vram_pct)?;
                log_writer.flush()?;
                log_counter = 0;
            }
            
            // Check for Ctrl+C (non-blocking)
            if event::poll(Duration::from_secs(1))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        running.store(false, Ordering::Relaxed);
                    }
                }
            }
        }
        
        // Cleanup terminal
        execute!(stdout, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
    }
    
    // Print session summary
    println!("\nSession Summary:");
    println!("Peak GPU: {}% | CPU: {}% | RAM: {}%", max.gpu, max.cpu, max.ram);
    let max_vram_pct = if max.gpu_mem_total > 0 { (max.gpu_mem * 100) / max.gpu_mem_total } else { 0 };
    println!("Peak Temp: {}°C | VRAM: {} MiB ({}%)", max.gpu_temp, max.gpu_mem, max_vram_pct);
    if gpu95_hit { println!("⚠ GPU hit 95%+"); }
    println!("Log: {}", log_path.display());
    
    Ok(())
}