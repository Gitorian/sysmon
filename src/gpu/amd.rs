use adlx::{Adlx, AdlxGpu};

pub struct AdlxContext {
    adlx: Adlx,
}

impl AdlxContext {
    pub fn new() -> Option<Self> {
        Adlx::new().ok().map(|adlx| Self { adlx })
    }

    pub fn get_metrics(&self) -> (u32, u32, u32, u32) {
        let gpus = match self.adlx.get_gpus() {
            Ok(g) => g,
            Err(_) => return (0, 0, 0, 0),
        };

        if gpus.is_empty() {
            return (0, 0, 0, 0);
        }

        let gpu = &gpus[0];

        let util = gpu.gpu_usage().unwrap_or(0.0) as u32;
        let temp = gpu.gpu_temperature().unwrap_or(0.0) as u32;

        let vram_mb = gpu.vram_mb().unwrap_or(0);
        let total_vram_mb = gpu.total_vram_mb().unwrap_or(0);

        (util, temp, vram_mb, total_vram_mb)
    }
}
