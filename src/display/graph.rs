use std::collections::VecDeque;
use std::io::{self, Write};

use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Attribute, SetAttribute},
    terminal::{self, Clear, ClearType},
};

use crate::metrics::Metrics;

pub fn render_graph(
    stdout: &mut io::Stdout,
    m: &Metrics,
    max: &Metrics,
    gpu95: bool,
    history: &VecDeque<Metrics>,
    log_filename: &str,
    interval: u64,
    priority: &str,
    gpu_vendor: &str,
) -> io::Result<()> {
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    // Header
    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        SetAttribute(Attribute::Bold),
        Print("╔════════════════════════════════════════════════╗\n"),
        Print("║            sysmon - GRAPH VIEW                 ║\n"),
        Print("╚════════════════════════════════════════════════╝\n"),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;

    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print("GPU: "),
        SetForegroundColor(Color::Green),
        SetAttribute(Attribute::Bold),
        Print(format!("{gpu_vendor}")),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(Color::DarkGrey),
        Print(format!(" │ Interval: {interval}s │ Priority: {priority}\n")),
        ResetColor
    )?;

    let (tw, th) = terminal::size()?;
    let gw = (tw as usize).saturating_sub(12).max(40);
    let gh = (th as usize).saturating_sub(18).max(15);

    // Determine Y-axis scale based on max value in history
    let max_val = history
        .iter()
        .map(|m| m.gpu.max(m.cpu).max(m.ram).max(m.vram_percent()))
        .max()
        .unwrap_or(100);
    let yscale = if max_val <= 50 {
        50
    } else if max_val <= 75 {
        75
    } else {
        100
    };

    // Top border with dots
    println!();
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print(format!("{:3}% ┤", yscale)),
        ResetColor
    )?;
    for _ in 0..gw {
        execute!(stdout, SetForegroundColor(Color::DarkGrey), Print("·"), ResetColor)?;
    }
    println!();

    // Calculate sampling step for downsampling if needed
    let step = if history.len() <= gw {
        1
    } else {
        (history.len() * 10) / gw
    };

    // Render each horizontal line of the graph
    for y in (0..gh).rev() {
        let percent = (y * yscale as usize) / gh;

        // Y-axis labels with midpoint reference line
        if percent == yscale as usize / 2 {
            execute!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("{:3}% ┼", percent)),
                ResetColor
            )?;
        } else {
            execute!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("{:3}% │", percent)),
                ResetColor
            )?;
        }

        // Plot each data point
        for x in 0..gw.min(history.len()) {
            let idx = if step == 1 { x } else { (x * step) / 10 };
            let s = history[idx.min(history.len() - 1)];

            let gy = ((s.gpu as usize * gh) / yscale as usize).min(gh);
            let cy = ((s.cpu as usize * gh) / yscale as usize).min(gh);
            let ry = ((s.ram as usize * gh) / yscale as usize).min(gh);
            let vy = ((s.vram_percent() as usize * gh) / yscale as usize).min(gh);

            // Layer metrics: GPU on top, then VRAM, CPU, RAM
            if gy >= y {
                execute!(stdout, SetForegroundColor(Color::Green), Print("█"), ResetColor)?;
            } else if vy >= y {
                execute!(stdout, SetForegroundColor(Color::Cyan), Print("▓"), ResetColor)?;
            } else if cy >= y {
                execute!(stdout, SetForegroundColor(Color::Blue), Print("▒"), ResetColor)?;
            } else if ry >= y {
                execute!(stdout, SetForegroundColor(Color::Magenta), Print("░"), ResetColor)?;
            } else {
                // Background grid for readability
                if percent % 25 == 0 && x % 10 == 0 {
                    execute!(stdout, SetForegroundColor(Color::DarkGrey), Print("·"), ResetColor)?;
                } else {
                    print!(" ");
                }
            }
        }
        println!();
    }

    // X-axis with tick marks every 10 columns
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print("  0% └"),
        ResetColor
    )?;
    for i in 0..gw {
        if i % 10 == 0 {
            execute!(stdout, SetForegroundColor(Color::DarkGrey), Print("┴"), ResetColor)?;
        } else {
            execute!(stdout, SetForegroundColor(Color::DarkGrey), Print("─"), ResetColor)?;
        }
    }
    println!();

    // Time information
    let time_ago = history.len() as u64 * interval;
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print(format!("      ←{} samples (~{}s ago)", history.len(), time_ago)),
        ResetColor
    )?;
    println!("\n");

    // Legend
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print("     Legend: "),
        ResetColor,
        SetForegroundColor(Color::Green),
        SetAttribute(Attribute::Bold),
        Print("█"),
        SetAttribute(Attribute::Reset),
        Print(" GPU  "),
        ResetColor,
        SetForegroundColor(Color::Cyan),
        Print("▓"),
        Print(" VRAM  "),
        ResetColor,
        SetForegroundColor(Color::Blue),
        Print("▒"),
        Print(" CPU  "),
        ResetColor,
        SetForegroundColor(Color::Magenta),
        Print("░"),
        Print(" RAM"),
        ResetColor
    )?;

    if gpu95 {
        execute!(
            stdout,
            Print("  "),
            SetForegroundColor(Color::Yellow),
            SetAttribute(Attribute::Bold),
            Print("⚠"),
            SetAttribute(Attribute::Reset),
            Print(" GPU≥95%"),
            ResetColor
        )?;
    }
    println!("\n");

    // Current metrics
    execute!(
        stdout,
        SetForegroundColor(Color::White),
        SetAttribute(Attribute::Bold),
        Print("┌─ Current ────────────────────────────────────────────────────┐\n│ "),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;

    execute!(
        stdout,
        SetForegroundColor(Color::Green),
        Print(format!("GPU:{:3}%", m.gpu)),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(" │ "),
        ResetColor,
        SetForegroundColor(Color::Blue),
        Print(format!("CPU:{:3}%", m.cpu)),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(" │ "),
        ResetColor,
        SetForegroundColor(Color::Cyan),
        Print(format!("VRAM:{:4}MB ({:2}%)", m.gpu_mem, m.vram_percent())),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(" │ "),
        ResetColor,
        SetForegroundColor(Color::Red),
        Print(format!("Temp:{:2}°C", m.gpu_temp)),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(" │ "),
        ResetColor,
        SetForegroundColor(Color::Magenta),
        Print(format!("RAM:{:3}%", m.ram)),
        ResetColor,
        Print(" │\n")
    )?;

    // Maximum metrics
    execute!(
        stdout,
        SetForegroundColor(Color::White),
        Print("├─ Maximum ────────────────────────────────────────────────────┤\n│ "),
        ResetColor
    )?;

    execute!(
        stdout,
        SetForegroundColor(Color::Green),
        Print(format!("GPU:{:3}%", max.gpu)),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(" │ "),
        ResetColor,
        SetForegroundColor(Color::Blue),
        Print(format!("CPU:{:3}%", max.cpu)),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(" │ "),
        ResetColor,
        SetForegroundColor(Color::Cyan),
        Print(format!("VRAM:{:4}MB ({:2}%)", max.gpu_mem, max.vram_percent())),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(" │ "),
        ResetColor,
        SetForegroundColor(Color::Red),
        Print(format!("Temp:{:2}°C", max.gpu_temp)),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(" │ "),
        ResetColor,
        SetForegroundColor(Color::Magenta),
        Print(format!("RAM:{:3}%", max.ram)),
        ResetColor,
        Print(" │\n")
    )?;

    execute!(
        stdout,
        SetForegroundColor(Color::White),
        Print("└──────────────────────────────────────────────────────────────┘\n"),
        ResetColor
    )?;

    println!();
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print("Press "),
        SetForegroundColor(Color::Yellow),
        Print("Ctrl+C"),
        SetForegroundColor(Color::DarkGrey),
        Print(" to quit │ Log: "),
        SetForegroundColor(Color::Cyan),
        Print(log_filename),
        ResetColor
    )?;

    stdout.flush()
}
