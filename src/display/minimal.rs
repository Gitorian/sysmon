use std::io::{self, Write};

use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Attribute, SetAttribute},
    terminal::{Clear, ClearType},
};

use crate::metrics::Metrics;

const GPU_WARNING_THRESHOLD: u32 = 95;

pub fn render_minimal(
    stdout: &mut io::Stdout,
    m: &Metrics,
    max: &Metrics,
    _warning: bool,
    gpu95: bool,
    log_filename: &str,
    interval: u64,
    priority: &str,
    gpu_vendor: &str,
) -> io::Result<()> {
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;

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

    println!();

    // Current metrics
    execute!(
        stdout,
        SetForegroundColor(Color::White),
        SetAttribute(Attribute::Bold),
        Print("┌─ Current ──────────────────────────────────────────────────────┐\n│ "),
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
        Print("├─ Maximum ──────────────────────────────────────────────────────┤\n│ "),
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
        Print("└────────────────────────────────────────────────────────────────┘\n"),
        ResetColor
    )?;

    // Show warning if GPU hit threshold
    if gpu95 {
        println!();
        execute!(
            stdout,
            SetForegroundColor(Color::Yellow),
            SetAttribute(Attribute::Bold),
            Print("⚠"),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Yellow),
            Print(format!(" Warning: GPU reached {GPU_WARNING_THRESHOLD}%+ during session\n")),
            ResetColor
        )?;
    }

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
