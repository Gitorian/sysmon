use std::fs::OpenOptions;
use std::io::{self, Write, BufWriter};
use std::path::PathBuf;
use std::env;
use anyhow::Result;
use chrono::Local;

use crate::metrics::Metrics;

const LOG_BUFFER_SIZE: usize = 16 * 1024;

pub struct Logger {
    writer: BufWriter<std::fs::File>,
    filename: String,
    write_count: usize,
    buffer: String,
}

impl Logger {
    pub fn new() -> Result<Self> {
        let exe_dir = env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let filename = format!("sysmon_{}.csv", Local::now().format("%Y-%m-%d_%H-%M-%S"));
        let log_path = exe_dir.join(&filename);

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let mut writer = BufWriter::with_capacity(LOG_BUFFER_SIZE, log_file);
        writeln!(writer, "Timestamp,GPU%,CPU%,RAM%,GPU_Temp,GPU_Mem_MiB,GPU_Mem_%")?;

        Ok(Self {
            writer,
            filename,
            write_count: 0,
            buffer: String::with_capacity(128),
        })
    }

    pub fn log(&mut self, metrics: &Metrics) -> io::Result<()> {
        use std::fmt::Write;

        // Reuse buffer to avoid allocations
        self.buffer.clear();
        let _ = write!(
            &mut self.buffer,
            "{},{},{},{},{},{},{}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            metrics.gpu,
            metrics.cpu,
            metrics.ram,
            metrics.gpu_temp,
            metrics.gpu_mem,
            metrics.vram_percent()
        );

        writeln!(self.writer, "{}", self.buffer)?;

        self.write_count += 1;

        // Flush every 10 writes to balance responsiveness vs I/O overhead
        if self.write_count >= 10 {
            self.writer.flush()?;
            self.write_count = 0;
        }

        Ok(())
    }

    #[must_use]
    pub fn filename(&self) -> &str {
        &self.filename
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        let _ = self.writer.flush();
    }
}
