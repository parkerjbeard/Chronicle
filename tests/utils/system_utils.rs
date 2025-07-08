use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::path::Path;
use anyhow::Result;
use serde::{Serialize, Deserialize};

/// System utilities for Chronicle tests
pub struct SystemUtils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_cores: usize,
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub disk_space_gb: u64,
    pub load_average: f64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub threads: u32,
    pub open_files: u32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub interface: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub errors: u64,
}

impl SystemUtils {
    /// Get comprehensive system information
    pub fn get_system_info() -> Result<SystemInfo> {
        Ok(SystemInfo {
            os: Self::get_os_info()?,
            arch: Self::get_architecture(),
            cpu_cores: Self::get_cpu_count(),
            total_memory_mb: Self::get_total_memory()?,
            available_memory_mb: Self::get_available_memory()?,
            disk_space_gb: Self::get_disk_space()?,
            load_average: Self::get_load_average()?,
            uptime_seconds: Self::get_uptime()?,
        })
    }

    /// Get current process information
    pub fn get_process_info(pid: Option<u32>) -> Result<ProcessInfo> {
        let pid = pid.unwrap_or_else(|| std::process::id());
        
        #[cfg(target_os = "macos")]
        {
            Self::get_process_info_macos(pid)
        }
        
        #[cfg(target_os = "linux")]
        {
            Self::get_process_info_linux(pid)
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok(ProcessInfo {
                pid,
                name: "unknown".to_string(),
                cpu_percent: 0.0,
                memory_mb: 0,
                threads: 1,
                open_files: 0,
                status: "unknown".to_string(),
            })
        }
    }

    /// Monitor process resources over time
    pub async fn monitor_process(
        pid: u32,
        duration: Duration,
        interval: Duration,
    ) -> Result<Vec<ProcessInfo>> {
        let mut samples = Vec::new();
        let start = Instant::now();
        
        while start.elapsed() < duration {
            if let Ok(info) = Self::get_process_info(Some(pid)) {
                samples.push(info);
            }
            
            tokio::time::sleep(interval).await;
        }
        
        Ok(samples)
    }

    /// Check if a process is running
    pub fn is_process_running(pid: u32) -> bool {
        #[cfg(unix)]
        {
            use nix::sys::signal::{self, Signal};
            use nix::unistd::Pid;
            
            match signal::kill(Pid::from_raw(pid as i32), Signal::SIGCONT) {
                Ok(_) => true,
                Err(_) => false,
            }
        }
        
        #[cfg(windows)]
        {
            // Windows implementation would go here
            false
        }
        
        #[cfg(not(any(unix, windows)))]
        {
            false
        }
    }

    /// Get network interface information
    pub fn get_network_info() -> Result<Vec<NetworkInfo>> {
        let mut interfaces = Vec::new();
        
        #[cfg(target_os = "linux")]
        {
            let proc_net = std::fs::read_to_string("/proc/net/dev")?;
            for line in proc_net.lines().skip(2) {
                if let Some(info) = Self::parse_linux_network_line(line) {
                    interfaces.push(info);
                }
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("netstat")
                .args(&["-i", "-b"])
                .output()?;
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            for line in output_str.lines().skip(1) {
                if let Some(info) = Self::parse_macos_network_line(line) {
                    interfaces.push(info);
                }
            }
        }
        
        Ok(interfaces)
    }

    /// Execute command with timeout
    pub fn execute_command_with_timeout(
        command: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<String> {
        let mut cmd = Command::new(command);
        cmd.args(args)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let child = cmd.spawn()?;
        
        let start = Instant::now();
        let output = loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let output = child.wait_with_output()?;
                    if status.success() {
                        break String::from_utf8_lossy(&output.stdout).to_string();
                    } else {
                        return Err(anyhow::anyhow!(
                            "Command failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        return Err(anyhow::anyhow!("Command timed out"));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(e.into()),
            }
        };
        
        Ok(output)
    }

    /// Check if running in CI environment
    pub fn is_ci_environment() -> bool {
        std::env::var("CI").is_ok() ||
        std::env::var("GITHUB_ACTIONS").is_ok() ||
        std::env::var("GITLAB_CI").is_ok() ||
        std::env::var("JENKINS_URL").is_ok() ||
        std::env::var("TRAVIS").is_ok()
    }

    /// Get CI environment type
    pub fn get_ci_environment() -> Option<String> {
        if std::env::var("GITHUB_ACTIONS").is_ok() {
            Some("github".to_string())
        } else if std::env::var("GITLAB_CI").is_ok() {
            Some("gitlab".to_string())
        } else if std::env::var("JENKINS_URL").is_ok() {
            Some("jenkins".to_string())
        } else if std::env::var("TRAVIS").is_ok() {
            Some("travis".to_string())
        } else if std::env::var("CI").is_ok() {
            Some("generic".to_string())
        } else {
            None
        }
    }

    /// Check available disk space for a path
    pub fn check_disk_space<P: AsRef<Path>>(path: P) -> Result<u64> {
        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::mem;
            
            let path_cstr = CString::new(path.as_ref().to_string_lossy().as_bytes())?;
            let mut statvfs: libc::statvfs = unsafe { mem::zeroed() };
            
            let result = unsafe {
                libc::statvfs(path_cstr.as_ptr(), &mut statvfs)
            };
            
            if result == 0 {
                let available_bytes = statvfs.f_bavail * statvfs.f_frsize;
                Ok(available_bytes / (1024 * 1024 * 1024)) // Convert to GB
            } else {
                Err(anyhow::anyhow!("Failed to get disk space"))
            }
        }
        
        #[cfg(not(unix))]
        {
            Ok(0) // Placeholder for non-Unix systems
        }
    }

    /// Kill process and all its children
    pub fn kill_process_tree(pid: u32) -> Result<()> {
        #[cfg(unix)]
        {
            // Kill the process group
            let _ = Command::new("pkill")
                .args(&["-P", &pid.to_string()])
                .output();
            
            // Kill the main process
            use nix::sys::signal::{self, Signal};
            use nix::unistd::Pid;
            
            signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
                .map_err(|e| anyhow::anyhow!("Failed to kill process: {}", e))?;
        }
        
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(&["/F", "/T", "/PID", &pid.to_string()])
                .output();
        }
        
        Ok(())
    }

    // Platform-specific implementations
    
    fn get_os_info() -> Result<String> {
        Ok(format!("{} {}", std::env::consts::OS, Self::get_os_version()?))
    }

    fn get_os_version() -> Result<String> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("sw_vers")
                .args(&["-productVersion"])
                .output()?;
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("VERSION_ID=") {
                        return Ok(line.split('=').nth(1).unwrap_or("unknown").trim_matches('"').to_string());
                    }
                }
            }
            Ok("unknown".to_string())
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok("unknown".to_string())
        }
    }

    fn get_architecture() -> String {
        std::env::consts::ARCH.to_string()
    }

    fn get_cpu_count() -> usize {
        num_cpus::get()
    }

    fn get_total_memory() -> Result<u64> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("sysctl")
                .args(&["-n", "hw.memsize"])
                .output()?;
            let memory_bytes: u64 = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse()?;
            Ok(memory_bytes / (1024 * 1024)) // Convert to MB
        }
        
        #[cfg(target_os = "linux")]
        {
            let meminfo = std::fs::read_to_string("/proc/meminfo")?;
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let kb: u64 = parts[1].parse()?;
                        return Ok(kb / 1024); // Convert to MB
                    }
                }
            }
            Err(anyhow::anyhow!("Could not parse memory info"))
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok(0)
        }
    }

    fn get_available_memory() -> Result<u64> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("vm_stat").output()?;
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse vm_stat output (simplified)
            let mut free_pages = 0u64;
            for line in output_str.lines() {
                if line.contains("Pages free:") {
                    if let Some(pages_str) = line.split(':').nth(1) {
                        if let Ok(pages) = pages_str.trim().trim_end_matches('.').parse::<u64>() {
                            free_pages = pages;
                            break;
                        }
                    }
                }
            }
            
            // Assume 4KB pages
            Ok(free_pages * 4 / 1024) // Convert to MB
        }
        
        #[cfg(target_os = "linux")]
        {
            let meminfo = std::fs::read_to_string("/proc/meminfo")?;
            for line in meminfo.lines() {
                if line.starts_with("MemAvailable:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let kb: u64 = parts[1].parse()?;
                        return Ok(kb / 1024); // Convert to MB
                    }
                }
            }
            Err(anyhow::anyhow!("Could not parse available memory"))
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok(0)
        }
    }

    fn get_disk_space() -> Result<u64> {
        #[cfg(unix)]
        {
            let output = Command::new("df")
                .args(&["-h", "/"])
                .output()?;
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse df output
            for line in output_str.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let available = parts[3];
                    // Convert from human readable format (simplified)
                    if let Some(size_str) = available.strip_suffix('G') {
                        if let Ok(size) = size_str.parse::<u64>() {
                            return Ok(size);
                        }
                    }
                }
            }
            Ok(0)
        }
        
        #[cfg(not(unix))]
        {
            Ok(0)
        }
    }

    fn get_load_average() -> Result<f64> {
        #[cfg(unix)]
        {
            let output = Command::new("uptime").output()?;
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse load average from uptime output
            if let Some(load_part) = output_str.split("load average:").nth(1) {
                if let Some(first_load) = load_part.split(',').next() {
                    if let Ok(load) = first_load.trim().parse::<f64>() {
                        return Ok(load);
                    }
                }
            }
            Ok(0.0)
        }
        
        #[cfg(not(unix))]
        {
            Ok(0.0)
        }
    }

    fn get_uptime() -> Result<u64> {
        #[cfg(target_os = "linux")]
        {
            let uptime_str = std::fs::read_to_string("/proc/uptime")?;
            let uptime_parts: Vec<&str> = uptime_str.split_whitespace().collect();
            if let Some(uptime_str) = uptime_parts.first() {
                if let Ok(uptime) = uptime_str.parse::<f64>() {
                    return Ok(uptime as u64);
                }
            }
            Err(anyhow::anyhow!("Could not parse uptime"))
        }
        
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("sysctl")
                .args(&["-n", "kern.boottime"])
                .output()?;
            // Parse boot time and calculate uptime (simplified)
            Ok(0) // Placeholder
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok(0)
        }
    }

    #[cfg(target_os = "macos")]
    fn get_process_info_macos(pid: u32) -> Result<ProcessInfo> {
        let output = Command::new("ps")
            .args(&["-p", &pid.to_string(), "-o", "pid,comm,%cpu,rss,nlwp,stat"])
            .output()?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                return Ok(ProcessInfo {
                    pid,
                    name: parts[1].to_string(),
                    cpu_percent: parts[2].parse().unwrap_or(0.0),
                    memory_mb: parts[3].parse::<u64>().unwrap_or(0) / 1024, // RSS is in KB
                    threads: parts[4].parse().unwrap_or(1),
                    open_files: 0, // Would need lsof to get this
                    status: parts[5].to_string(),
                });
            }
        }
        
        Err(anyhow::anyhow!("Process not found"))
    }

    #[cfg(target_os = "linux")]
    fn get_process_info_linux(pid: u32) -> Result<ProcessInfo> {
        let stat_path = format!("/proc/{}/stat", pid);
        let status_path = format!("/proc/{}/status", pid);
        
        let stat_content = std::fs::read_to_string(&stat_path)?;
        let status_content = std::fs::read_to_string(&status_path)?;
        
        // Parse /proc/pid/stat (simplified)
        let stat_parts: Vec<&str> = stat_content.split_whitespace().collect();
        let name = if stat_parts.len() > 1 {
            stat_parts[1].trim_matches(|c| c == '(' || c == ')').to_string()
        } else {
            "unknown".to_string()
        };
        
        // Parse /proc/pid/status for additional info
        let mut memory_mb = 0;
        let mut threads = 1;
        let mut status = "unknown".to_string();
        
        for line in status_content.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    memory_mb = value.parse::<u64>().unwrap_or(0) / 1024; // Convert KB to MB
                }
            } else if line.starts_with("Threads:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    threads = value.parse().unwrap_or(1);
                }
            } else if line.starts_with("State:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    status = value.to_string();
                }
            }
        }
        
        Ok(ProcessInfo {
            pid,
            name,
            cpu_percent: 0.0, // Would need multiple samples to calculate
            memory_mb,
            threads,
            open_files: 0, // Would need to count /proc/pid/fd entries
            status,
        })
    }

    #[cfg(target_os = "linux")]
    fn parse_linux_network_line(line: &str) -> Option<NetworkInfo> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 17 {
            let interface = parts[0].trim_end_matches(':').to_string();
            Some(NetworkInfo {
                interface,
                bytes_received: parts[1].parse().unwrap_or(0),
                packets_received: parts[2].parse().unwrap_or(0),
                errors: parts[3].parse().unwrap_or(0),
                bytes_sent: parts[9].parse().unwrap_or(0),
                packets_sent: parts[10].parse().unwrap_or(0),
            })
        } else {
            None
        }
    }

    #[cfg(target_os = "macos")]
    fn parse_macos_network_line(line: &str) -> Option<NetworkInfo> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 7 {
            Some(NetworkInfo {
                interface: parts[0].to_string(),
                bytes_received: parts[6].parse().unwrap_or(0),
                bytes_sent: parts[9].parse().unwrap_or(0),
                packets_received: parts[4].parse().unwrap_or(0),
                packets_sent: parts[7].parse().unwrap_or(0),
                errors: parts[5].parse().unwrap_or(0),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_system_info() {
        let info = SystemUtils::get_system_info().unwrap();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.cpu_cores > 0);
    }

    #[test]
    fn test_get_process_info() {
        let info = SystemUtils::get_process_info(None).unwrap();
        assert_eq!(info.pid, std::process::id());
        assert!(!info.name.is_empty());
    }

    #[test]
    fn test_is_process_running() {
        let current_pid = std::process::id();
        assert!(SystemUtils::is_process_running(current_pid));
        assert!(!SystemUtils::is_process_running(999999)); // Unlikely to exist
    }

    #[test]
    fn test_ci_environment_detection() {
        // This will depend on the actual environment
        let is_ci = SystemUtils::is_ci_environment();
        let ci_type = SystemUtils::get_ci_environment();
        
        if is_ci {
            assert!(ci_type.is_some());
        }
    }

    #[test]
    fn test_execute_command_with_timeout() {
        let result = SystemUtils::execute_command_with_timeout(
            "echo",
            &["hello"],
            Duration::from_secs(5)
        );
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello");
    }
}