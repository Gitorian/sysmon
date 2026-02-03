// sysmon - Lightweight system monitor for Windows
// https://github.com/Gitorian/sysmon

mod metrics;
mod gpu;
mod display;
mod logging;

use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style::{SetForegroundColor, ResetColor, SetAttribute},
};

use anyhow::Result;
use metrics::{Metrics, CpuTracker};
use gpu::GpuManager;
use display::{DisplayMode, render_minimal, render_graph, TerminalGuard};
use logging::Logger;

const COLLECTION_INTERVAL_MS: u64 = 1000;
const INPUT_POLL_INTERVAL_MS: u64 = 100;
const MAX_HISTORY_POINTS: usize = 300;
const GPU_WARNING_THRESHOLD: u32 = 95;

fn print_menu(text: &str, color: crossterm::style::Color) -> io::Result<()> {
    execute!(
        io::stdout(),
        SetForegroundColor(color)
    )?;
    println!("{text}");
    execute!(io::stdout(), ResetColor)?;
    Ok(())
}

fn get_priority_name(priority: i32) -> &'static str {
    match priority {
        -2 => "Idle",
        -1 => "Below Normal",
        0 => "Normal",
        1 => "Above Normal",
        2 => "Highest",
        15 => "Time Critical",
        _ => "Unknown",
    }
}

fn select_display_mode() -> Result<DisplayMode> {
    print_menu("Select display mode:", crossterm::style::Color::White)?;
    print_menu("1 = Minimal", crossterm::style::Color::Green)?;
    print_menu("2 = Graph", crossterm::style::Color::Yellow)?;
    print_menu("3 = Headless (background logging)\n", crossterm::style::Color::DarkGrey)?;
    print!("Choice (1/2/3): ");
    io::stdout().flush()?;

    let mut input = String::with_capacity(16);
    io::stdin().read_line(&mut input)?;

    Ok(match input.trim() {
        "2" => DisplayMode::Graph,
        "3" => DisplayMode::Headless,
        _ => DisplayMode::Minimal,
    })
}

fn select_interval(mode: &DisplayMode) -> Result<u64> {
    use crossterm::style::Color;
    println!();
    print_menu("Select interval:", Color::White)?;

    if matches!(mode, DisplayMode::Headless) {
        print_menu("1 = 1 second", Color::Yellow)?;
        print_menu("2 = 2 seconds", Color::Green)?;
        print_menu("3 = 5 seconds", Color::Cyan)?;
        print_menu("4 = 10 seconds", Color::Magenta)?;
        print_menu("5 = 30 seconds\n", Color::DarkGrey)?;
        print!("Choice (1/2/3/4/5): ");
    } else {
        print_menu("1 = 1 second", Color::Yellow)?;
        print_menu("2 = 2 seconds", Color::Green)?;
        print_menu("3 = 5 seconds\n", Color::Cyan)?;
        print!("Choice (1/2/3): ");
    }

    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(match input.trim() {
        "2" => 2,
        "3" => 5,
        "4" => 10,
        "5" => 30,
        _ => 1,
    })
}

#[inline]
fn collect_metrics(cpu_tracker: &mut CpuTracker, gpu_manager: &GpuManager) -> Metrics {
    let cpu = cpu_tracker.get_cpu_usage();
    let ram = metrics::get_ram_usage();
    let (gpu, gpu_temp, gpu_mem, gpu_mem_total) = gpu_manager.get_metrics();

    Metrics {
        cpu,
        ram,
        gpu,
        gpu_temp,
        gpu_mem,
        gpu_mem_total,
    }
}

fn run_headless(
    interval: u64,
    gpu_manager: &GpuManager,
    mut cpu_tracker: CpuTracker,
    mut logger: Logger,
    running: Arc<AtomicBool>,
) -> Result<Metrics> {
    let mut max = Metrics::default();
    let mut next_log = Instant::now();
    let log_interval = Duration::from_secs(interval);
    let mut log_count = 0u32;

    while running.load(Ordering::Relaxed) {
        let m = collect_metrics(&mut cpu_tracker, gpu_manager);
        max.update_max(&m);

        if Instant::now() >= next_log {
            logger.log(&m)?;
            log_count += 1;

            // Print status every 10 logs
            if log_count % 10 == 0 {
                println!(
                    "[{}] GPU:{}% CPU:{}% RAM:{}% T:{}°C",
                    chrono::Local::now().format("%H:%M:%S"),
                    m.gpu, m.cpu, m.ram, m.gpu_temp
                );
            }

            next_log += log_interval;
        }

        std::thread::sleep(Duration::from_millis(COLLECTION_INTERVAL_MS));
    }

    Ok(max)
}

fn run_interactive(
    mode: DisplayMode,
    interval: u64,
    gpu_manager: &GpuManager,
    mut cpu_tracker: CpuTracker,
    mut logger: Logger,
    running: Arc<AtomicBool>,
    priority_name: &str,
    log_filename: &str,
) -> Result<Metrics> {
    let mut stdout = io::stdout();
    let _guard = TerminalGuard::new()?;

    let mut max = Metrics::default();
    let mut gpu95_hit = false;
    let mut next_update = Instant::now();
    let update_interval = Duration::from_secs(interval);

    // Only allocate history for graph mode
    let mut history = if matches!(mode, DisplayMode::Graph) {
        VecDeque::with_capacity(MAX_HISTORY_POINTS)
    } else {
        VecDeque::new()
    };

    while running.load(Ordering::Relaxed) {
        let m = collect_metrics(&mut cpu_tracker, gpu_manager);
        max.update_max(&m);

        let warning = m.gpu >= GPU_WARNING_THRESHOLD;
        if warning {
            gpu95_hit = true;
        }

        // Store history for graph mode
        if matches!(mode, DisplayMode::Graph) {
            if history.len() >= MAX_HISTORY_POINTS {
                history.pop_front();
            }
            history.push_back(m);
        }

        // Update display at configured interval
        if Instant::now() >= next_update {
            match mode {
                DisplayMode::Minimal => render_minimal(
                    &mut stdout,
                    &m,
                    &max,
                    warning,
                    gpu95_hit,
                    log_filename,
                    interval,
                    priority_name,
                    gpu_manager.vendor_name(),
                )?,
                DisplayMode::Graph => render_graph(
                    &mut stdout,
                    &m,
                    &max,
                    gpu95_hit,
                    &history,
                    log_filename,
                    interval,
                    priority_name,
                    gpu_manager.vendor_name(),
                )?,
                _ => {}
            }

            logger.log(&m)?;
            next_update += update_interval;
        }

        // Poll for Ctrl+C
        for _ in 0..(COLLECTION_INTERVAL_MS / INPUT_POLL_INTERVAL_MS) {
            if event::poll(Duration::from_millis(INPUT_POLL_INTERVAL_MS))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        running.store(false, Ordering::Relaxed);
                        break;
                    }
                }
            }
            if !running.load(Ordering::Relaxed) {
                break;
            }
        }
    }

    Ok(max)
}

fn main() -> Result<()> {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, GetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL
    };

    // Set thread priority to below normal
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }

    let priority = unsafe { GetThreadPriority(GetCurrentThread()) };
    let priority_name = get_priority_name(priority);
    let gpu_manager = GpuManager::auto_detect();

    // ASCII art banner
    execute!(
        io::stdout(),
        SetForegroundColor(crossterm::style::Color::Cyan),
        SetAttribute(crossterm::style::Attribute::Bold)
    )?;
    
    println!(r"
███████╗██╗   ██╗███████╗███╗   ███╗ ██████╗ ███╗   ██╗
██╔════╝╚██╗ ██╔╝██╔════╝████╗ ████║██╔═══██╗████╗  ██║
███████╗ ╚████╔╝ ███████╗██╔████╔██║██║   ██║██╔██╗ ██║
╚════██║  ╚██╔╝  ╚════██║██║╚██╔╝██║██║   ██║██║╚██╗██║
███████║   ██║   ███████║██║ ╚═╝ ██║╚██████╔╝██║ ╚████║
╚══════╝   ╚═╝   ╚══════╝╚═╝     ╚═╝ ╚═════╝ ╚═╝  ╚═══╝");
    
    execute!(
        io::stdout(),
        SetAttribute(crossterm::style::Attribute::Reset),
        ResetColor
    )?;
    
    println!("=======================================================\n");

    match gpu_manager.vendor() {
        gpu::GpuVendor::Nvidia => println!("✓ NVIDIA GPU detected (NVML)"),
        gpu::GpuVendor::Amd => println!("✓ AMD GPU detected (ADLX SDK)"),
        gpu::GpuVendor::Unknown => println!("⚠ No compatible GPU detected (running CPU/RAM only)"),
    }

    println!();
    let mode = select_display_mode()?;
    let interval = select_interval(&mode)?;

    println!("✓ Priority: {priority_name} ({priority})");

    let logger = Logger::new()?;
    let log_filename = logger.filename().to_string();
    println!("✓ Log: {}", log_filename);

    if matches!(mode, DisplayMode::Headless) {
        println!("\nRunning in headless mode...");
        println!("Logging every {interval}s to: {}", logger.filename());
        println!("Press Ctrl+C to stop\n");
    } else {
        println!("\nStarting... Press Ctrl+C to quit\n");
        std::thread::sleep(Duration::from_millis(500));
    }

    // Initialize CPU tracker with one reading
    let mut cpu_tracker = CpuTracker::new();
    cpu_tracker.get_cpu_usage();
    std::thread::sleep(Duration::from_millis(100));

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::Relaxed))?;

    let max = if matches!(mode, DisplayMode::Headless) {
        run_headless(interval, &gpu_manager, cpu_tracker, logger, running)?
    } else {
        run_interactive(mode, interval, &gpu_manager, cpu_tracker, logger, running, priority_name, &log_filename)?
    };

    println!("\n=======================================================");
    println!("Session Summary:");
    println!("Max GPU: {}%", max.gpu);
    println!("Max CPU: {}%", max.cpu);
    println!("Max RAM: {}%", max.ram);
    println!("Max GPU Temp: {}°C", max.gpu_temp);
    if max.gpu >= GPU_WARNING_THRESHOLD {
        println!("⚠ GPU reached {}%+ during session", GPU_WARNING_THRESHOLD);
    }
    println!("Log saved to: {}", log_filename);
    println!("=======================================================\n");

    Ok(())
}
