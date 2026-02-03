use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{GetSystemTimes, FILETIME};

#[derive(Copy, Clone, Default)]
pub struct Metrics {
    pub cpu: u32,
    pub ram: u32,
    pub gpu: u32,
    pub gpu_temp: u32,
    pub gpu_mem: u32,
    pub gpu_mem_total: u32,
}

impl Metrics {
    #[inline]
    pub fn vram_percent(&self) -> u32 {
        if self.gpu_mem_total == 0 {
            0
        } else {
            ((self.gpu_mem as u64 * 100) / self.gpu_mem_total as u64) as u32
        }
    }

    pub fn update_max(&mut self, other: &Metrics) {
        self.cpu = self.cpu.max(other.cpu);
        self.ram = self.ram.max(other.ram);
        self.gpu = self.gpu.max(other.gpu);
        self.gpu_temp = self.gpu_temp.max(other.gpu_temp);
        self.gpu_mem = self.gpu_mem.max(other.gpu_mem);
    }
}

pub struct CpuTracker {
    last_idle: u64,
    last_kernel: u64,
    last_user: u64,
}

impl CpuTracker {
    pub fn new() -> Self {
        Self {
            last_idle: 0,
            last_kernel: 0,
            last_user: 0,
        }
    }

    pub fn get_cpu_usage(&mut self) -> u32 {
        unsafe {
            let mut idle = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();

            if GetSystemTimes(
                Some(&mut idle as *mut _),
                Some(&mut kernel as *mut _),
                Some(&mut user as *mut _),
            )
            .is_err()
            {
                return 0;
            }

            let idle_time = filetime_to_u64(&idle);
            let kernel_time = filetime_to_u64(&kernel);
            let user_time = filetime_to_u64(&user);

            let idle_delta = idle_time.saturating_sub(self.last_idle);
            let kernel_delta = kernel_time.saturating_sub(self.last_kernel);
            let user_delta = user_time.saturating_sub(self.last_user);

            self.last_idle = idle_time;
            self.last_kernel = kernel_time;
            self.last_user = user_time;

            let sys_delta = kernel_delta + user_delta;
            if sys_delta == 0 {
                return 0;
            }

            let usage = (((sys_delta - idle_delta) * 100) / sys_delta) as u32;
            usage.min(100)
        }
    }
}

#[inline]
fn filetime_to_u64(ft: &FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

pub fn get_ram_usage() -> u32 {
    unsafe {
        let mut mem_info = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };

        if GlobalMemoryStatusEx(&mut mem_info).is_ok() {
            mem_info.dwMemoryLoad
        } else {
            0
        }
    }
}
