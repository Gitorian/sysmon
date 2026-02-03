use adlx::{helper::AdlxHelper, interface::Interface, gpu::Gpu1};
use super::{GpuBackend, GpuVendor};

pub struct AmdBackend {
    helper: AdlxHelper,
    cached_gpu: Option<Gpu1>,
}

impl AmdBackend {
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
}

impl GpuBackend for AmdBackend {
    fn vendor(&self) -> GpuVendor {
        GpuVendor::Amd
    }

    fn get_utilization(&self) -> u32 {
        if let Some(gpu) = &self.cached_gpu {
            let system = self.helper.system();
            if let Ok(perf_monitor) = system.performance_monitoring_services() {
                if let Ok(current_metrics) = perf_monitor.current_gpu_metrics(gpu) {
                    if let Ok(usage) = current_metrics.usage() {
                        return usage as u32;
                    }
                }
            }
        }
        0
    }

    fn get_temperature(&self) -> u32 {
        if let Some(gpu) = &self.cached_gpu {
            let system = self.helper.system();
            if let Ok(perf_monitor) = system.performance_monitoring_services() {
                if let Ok(current_metrics) = perf_monitor.current_gpu_metrics(gpu) {
                    if let Ok(temp) = current_metrics.temperature() {
                        return temp as u32;
                    }
                }
            }
        }
        0
    }

    fn get_memory_usage(&self) -> (u32, u32) {
        if let Some(gpu) = &self.cached_gpu {
            let system = self.helper.system();
            if let Ok(perf_monitor) = system.performance_monitoring_services() {
                if let Ok(current_metrics) = perf_monitor.current_gpu_metrics(gpu) {
                    if let Ok(used) = current_metrics.vram() {
                        let used_mb = used as u32;
                        // ADLX alpha doesn't expose separate total memory yet
                        return (used_mb, used_mb);
                    }
                }
            }
        }
        (0, 0)
    }
}
