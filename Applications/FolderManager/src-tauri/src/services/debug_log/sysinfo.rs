// Best-effort machine description for the debug log header; a failed probe reads "unknown".
const UNKNOWN: &str = "unknown";

pub struct SysInfo {
    pub os: String,
    pub ram: String,
    pub cpu: String,
    pub gpu: String,
}

fn or_unknown(value: String) -> String {
    if value.trim().is_empty() { UNKNOWN.to_string() } else { value.trim().to_string() }
}

fn ram_gb(bytes: Option<u64>) -> String {
    bytes.map(|b| format!("{:.1} GB", b as f64 / 1_073_741_824.0)).unwrap_or_else(|| UNKNOWN.to_string())
}

fn cpu_text(model: String, cores: Option<usize>) -> String {
    match cores {
        Some(n) => format!("{} ({n} logical cores)", or_unknown(model)),
        None => or_unknown(model),
    }
}

fn gpu_text(names: Vec<String>) -> String {
    let names: Vec<String> = names.into_iter().map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect();
    if names.is_empty() { "none detected".to_string() } else { names.join("; ") }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn parse_windows(text: &str) -> SysInfo {
    let get = |key: &str| {
        text.lines()
            .filter_map(|l| l.split_once('='))
            .find(|(k, _)| k.trim() == key)
            .map(|(_, v)| v.trim().to_string())
            .unwrap_or_default()
    };
    SysInfo {
        os: or_unknown(format!("{} {}", get("Caption"), get("Version"))),
        ram: ram_gb(get("Memory").parse().ok()),
        cpu: cpu_text(get("Cpu"), get("Cores").parse().ok()),
        gpu: if text.contains("Gpu=") { gpu_text(get("Gpu").split("; ").map(str::to_string).collect()) } else { UNKNOWN.to_string() },
    }
}

#[cfg(windows)]
pub fn collect() -> SysInfo {
    const SCRIPT: &str = "$o = Get-CimInstance Win32_OperatingSystem; $c = Get-CimInstance Win32_ComputerSystem; \
        $p = @(Get-CimInstance Win32_Processor); $g = @(Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name }) -join '; '; \
        \"Caption=$($o.Caption)\"; \"Version=$($o.Version)\"; \"Memory=$($c.TotalPhysicalMemory)\"; \
        \"Cpu=$($p[0].Name)\"; \"Cores=$(($p | Measure-Object NumberOfLogicalProcessors -Sum).Sum)\"; \"Gpu=$g\"";
    let text = crate::fs_safe::run_powershell(SCRIPT, &[], None, std::time::Duration::from_secs(15)).unwrap_or_default();
    parse_windows(&text)
}

#[cfg(unix)]
fn run(program: &str, args: &[&str]) -> String {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
pub fn collect() -> SysInfo {
    let read = |path: &str| std::fs::read_to_string(path).unwrap_or_default();
    let os_release = read("/etc/os-release");
    let name = os_release
        .lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_string())
        .unwrap_or_default();
    let meminfo = read("/proc/meminfo");
    let kb = meminfo
        .lines()
        .find_map(|l| l.strip_prefix("MemTotal:"))
        .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok());
    let cpuinfo = read("/proc/cpuinfo");
    let model = cpuinfo
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    let cores = cpuinfo.lines().filter(|l| l.starts_with("processor")).count();
    let gpus = run("lspci", &[])
        .lines()
        .filter(|l| l.contains("VGA") || l.contains("3D"))
        .map(|l| l.splitn(2, ": ").nth(1).unwrap_or(l).to_string())
        .collect();
    SysInfo {
        os: or_unknown(format!("{name} {}", run("uname", &["-r"]))),
        ram: ram_gb(kb.map(|k| k * 1024)),
        cpu: cpu_text(model, (cores > 0).then_some(cores)),
        gpu: gpu_text(gpus),
    }
}

#[cfg(target_os = "macos")]
pub fn collect() -> SysInfo {
    let gpus = run("system_profiler", &["SPDisplaysDataType"])
        .lines()
        .filter_map(|l| l.trim().strip_prefix("Chipset Model:").map(str::to_string))
        .collect();
    SysInfo {
        os: or_unknown(format!("macOS {}", run("sw_vers", &["-productVersion"]))),
        ram: ram_gb(run("sysctl", &["-n", "hw.memsize"]).parse().ok()),
        cpu: cpu_text(run("sysctl", &["-n", "machdep.cpu.brand_string"]), run("sysctl", &["-n", "hw.logicalcpu"]).parse().ok()),
        gpu: gpu_text(gpus),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powershell_key_values_become_the_header_fields() {
        let text = "Caption=Microsoft Windows 10 Pro\r\nVersion=10.0.19045\r\nMemory=17179869184\r\n\
            Cpu=Intel(R) Core(TM) i7-8700 CPU @ 3.20GHz\r\nCores=12\r\nGpu=NVIDIA GeForce GTX 1060; Microsoft Basic Display Adapter";
        let info = parse_windows(text);
        assert_eq!(info.os, "Microsoft Windows 10 Pro 10.0.19045");
        assert_eq!(info.ram, "16.0 GB");
        assert_eq!(info.cpu, "Intel(R) Core(TM) i7-8700 CPU @ 3.20GHz (12 logical cores)");
        assert_eq!(info.gpu, "NVIDIA GeForce GTX 1060; Microsoft Basic Display Adapter");
    }

    #[test]
    fn a_failed_probe_reads_unknown() {
        let info = parse_windows("");
        assert_eq!((info.os.as_str(), info.ram.as_str(), info.cpu.as_str()), (UNKNOWN, UNKNOWN, UNKNOWN));
        assert_eq!(info.gpu, UNKNOWN);
        assert_eq!(parse_windows("Gpu=").gpu, "none detected");
    }
}
