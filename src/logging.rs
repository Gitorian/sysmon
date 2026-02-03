use anyhow::Result;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::metrics::Metrics;

pub struct Logger {
    writer: BufWriter<File>,
    filename: String,
}

impl Logger {
    pub fn new() -> Result<Self> {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("sysmon_{}.csv", timestamp);

        let mut path = std::env::current_exe()?;
        path.pop();
        path.push(&filename);

        let file = File::create(&path)?;
        let mut writer = BufWriter::with_capacity(16384, file);

        writeln!(
            writer,
            "timestamp,gpu_util,gpu_temp,gpu_mem_mb,cpu_util,ram_util"
        )?;
        writer.flush()?;

        Ok(Self {
            writer,
            filename: path.to_string_lossy().to_string(),
        })
    }

    pub fn log(&mut self, m: &Metrics) -> Result<()> {
        writeln!(
            self.writer,
            "{},{},{},{},{},{}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            m.gpu,
            m.gpu_temp,
            m.gpu_mem,
            m.cpu,
            m.ram
        )?;
        Ok(())
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        let _ = self.writer.flush();
    }
}
