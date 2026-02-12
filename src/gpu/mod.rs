// auto-detect gpu vendor (nvidia or amd) and init appropriate backend

#[cfg(feature = "nvidia")]
mod nvidia;
#[cfg(feature = "amd")]
mod amd;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Unknown,
}

pub struct GpuManager {
    vendor: GpuVendor,
    #[cfg(feature = "nvidia")]
    nvml: Option<nvidia::NvmlContext>,
    #[cfg(feature = "amd")]
    adlx: Option<amd::AdlxContext>,
}

impl GpuManager {
    pub fn auto_detect() -> Self {
        // try nvidia first
        #[cfg(feature = "nvidia")]
        if let Some(nvml) = nvidia::NvmlContext::new() {
            return Self {
                vendor: GpuVendor::Nvidia,
                nvml: Some(nvml),
                #[cfg(feature = "amd")]
                adlx: None,
            };
        }

        // fallback to amd
        #[cfg(feature = "amd")]
        if let Some(adlx) = amd::AdlxContext::new() {
            return Self {
                vendor: GpuVendor::Amd,
                #[cfg(feature = "nvidia")]
                nvml: None,
                adlx: Some(adlx),
            };
        }

        // no gpu found
        Self {
            vendor: GpuVendor::Unknown,
            #[cfg(feature = "nvidia")]
            nvml: None,
            #[cfg(feature = "amd")]
            adlx: None,
        }
    }

    pub fn vendor(&self) -> GpuVendor {
        self.vendor
    }

    pub fn vendor_name(&self) -> &'static str {
        match self.vendor {
            GpuVendor::Nvidia => "NVIDIA",
            GpuVendor::Amd => "AMD",
            GpuVendor::Unknown => "None",
        }
    }

    pub fn get_metrics(&self) -> (u32, u32, u32, u32) {
        match self.vendor {
            #[cfg(feature = "nvidia")]
            GpuVendor::Nvidia => {
                if let Some(ref nvml) = self.nvml {
                    nvml.get_metrics()
                } else {
                    (0, 0, 0, 0)
                }
            }
            #[cfg(feature = "amd")]
            GpuVendor::Amd => {
                if let Some(ref adlx) = self.adlx {
                    adlx.get_metrics()
                } else {
                    (0, 0, 0, 0)
                }
            }
            _ => (0, 0, 0, 0),
        }
    }
}
