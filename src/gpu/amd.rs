use adlx::{helper::AdlxHelper, interface::Interface, gpu::Gpu1};

pub struct AdlxContext {
    helper: AdlxHelper,
    cached_gpu: Option<Gpu1>,
}

impl AdlxContext {
    pub fn new() -> Option<Self> {
        let mut backend = Self { 
            helper: AdlxHelper::new().ok()?,
            cached_gpu: None,
        };
        // Initialize GPU cache on creation
        backend.cached_gpu = backend.get_primary_gpu();
        Some(backend)
    }

    fn get_primary_gpu(&self) -> Option<Gpu1> {
        let system = self.helper.system();
        let gpu_list = system.gpus().ok()?;
        if gpu_list.size() > 0 {
            let gpu = gpu_list.at(0).ok()?;
            gpu.cast::<Gpu1>().ok()
        } else {
            None
        }
    }

    pub fn get_metrics(&self) -> (u32, u32, u32, u32) {
        let gpu = match &self.cached_gpu {
            Some(g) => g,
            None => return (0, 0, 0, 0),
        };

        let system = self.helper.system();
        let perf_monitor = match system.performance_monitoring_services() {
            Ok(pm) => pm,
            Err(_) => return (0, 0, 0, 0),
        };

        let current_metrics = match perf_monitor.current_gpu_metrics(gpu) {
            Ok(cm) => cm,
            Err(_) => return (0, 0, 0, 0),
        };

        let util = current_metrics.usage().unwrap_or(0.0) as u32;
        let temp = current_metrics.temperature().unwrap_or(0.0) as u32;
        let vram = current_metrics.vram().unwrap_or(0) as u32;

        // ADLX alpha doesn't expose separate total memory yet
        (util, temp, vram, vram)
    }
}
