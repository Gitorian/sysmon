use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;

pub struct NvmlContext {
    nvml: Nvml,
}

impl NvmlContext {
    pub fn new() -> Option<Self> {
        Nvml::init().ok().map(|nvml| Self { nvml })
    }

    pub fn get_metrics(&self) -> (u32, u32, u32, u32) {
        let device = match self.nvml.device_by_index(0) {
            Ok(d) => d,
            Err(_) => return (0, 0, 0, 0),
        };

        let util = device
            .utilization_rates()
            .map(|u| u.gpu)
            .unwrap_or(0);

        let temp = device
            .temperature(TemperatureSensor::Gpu)
            .unwrap_or(0);

        let mem_info = device.memory_info().ok();
        let (mem_used, mem_total) = mem_info
            .map(|m| ((m.used / 1_048_576) as u32, (m.total / 1_048_576) as u32))
            .unwrap_or((0, 0));

        (util, temp, mem_used, mem_total)
    }
}
