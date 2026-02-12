// terminal ui modes: minimal, graph, and headless

mod minimal;
mod graph;

pub use minimal::render_minimal;
pub use graph::render_graph;

use crossterm::terminal;
use std::io;

pub enum DisplayMode {
    Minimal,
    Graph,
    Headless,
}

// cleanup terminal on exit
pub struct TerminalGuard;

impl TerminalGuard {
    pub fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}
