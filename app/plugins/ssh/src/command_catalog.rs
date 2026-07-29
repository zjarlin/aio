//! 跨硬件平台的内置 SSH 只读监测命令。

use crate::contract::{COMMAND_KIND_MONITOR, UpsertSshCommandRequest};

/// 返回初始化模板使用的内置命令目录。
pub fn builtin_commands() -> Vec<UpsertSshCommandRequest> {
    vec![
        command(
            "system-identity",
            "系统与硬件身份",
            "系统",
            "generic",
            "command -v uname >/dev/null 2>&1",
            "uname -a; printf '\n--- os-release ---\n'; cat /etc/os-release 2>/dev/null || true; printf '\n--- dmi ---\n'; for f in sys_vendor product_name product_version board_name; do printf '%s=' \"$f\"; cat \"/sys/class/dmi/id/$f\" 2>/dev/null || true; done",
            10,
        ),
        command(
            "cpu-topology",
            "CPU 拓扑与负载",
            "处理器",
            "generic",
            "command -v lscpu >/dev/null 2>&1",
            "lscpu; printf '\n--- load ---\n'; uptime; printf '\n--- top processes ---\n'; ps -eo pid,user,pcpu,pmem,comm,args --sort=-pcpu | head -25",
            20,
        ),
        command(
            "memory-usage",
            "内存使用与 DIMM",
            "内存",
            "generic",
            "command -v free >/dev/null 2>&1",
            "free -h; if command -v dmidecode >/dev/null 2>&1; then printf '\n--- dimm ---\n'; (sudo -n dmidecode -t memory 2>/dev/null || dmidecode -t memory 2>/dev/null || true) | sed -n '1,260p'; fi",
            30,
        ),
        command(
            "thermal-sensors",
            "温度、电压与风扇",
            "传感器",
            "generic",
            "command -v sensors >/dev/null 2>&1 || test -d /sys/class/hwmon",
            "if command -v sensors >/dev/null 2>&1; then sensors; fi; printf '\n--- hwmon ---\n'; for h in /sys/class/hwmon/hwmon*; do test -d \"$h\" || continue; printf '%s ' \"$h\"; cat \"$h/name\" 2>/dev/null || true; for f in \"$h\"/temp*_input \"$h\"/fan*_input \"$h\"/power*_input; do test -e \"$f\" || continue; printf '%s=' \"$(basename \"$f\")\"; cat \"$f\" 2>/dev/null || true; done; done",
            40,
        ),
        command(
            "hygon-dcu",
            "海光 DCU/HCU 状态",
            "加速卡",
            "hygon_dcu",
            "test -x /usr/local/hyhal/bin/hy-smi || command -v hy-smi >/dev/null 2>&1",
            "if test -x /usr/local/hyhal/bin/hy-smi; then /usr/local/hyhal/bin/hy-smi --showallinfo; else hy-smi --showallinfo; fi",
            50,
        ),
        command(
            "nvidia-gpu",
            "NVIDIA GPU 状态",
            "加速卡",
            "nvidia_gpu",
            "command -v nvidia-smi >/dev/null 2>&1",
            "nvidia-smi --query-gpu=index,name,uuid,temperature.gpu,power.draw,power.limit,memory.total,memory.used,utilization.gpu,driver_version --format=csv,noheader,nounits; printf '\n--- processes ---\n'; nvidia-smi pmon -c 1 2>/dev/null || true",
            60,
        ),
        command(
            "amd-rocm-gpu",
            "AMD ROCm GPU 状态",
            "加速卡",
            "amd_rocm",
            "command -v rocm-smi >/dev/null 2>&1",
            "rocm-smi --showproductname --showtemp --showpower --showmeminfo vram --showuse --showdriverversion",
            70,
        ),
        command(
            "intel-xpu",
            "Intel XPU 状态",
            "加速卡",
            "intel_xpu",
            "command -v xpu-smi >/dev/null 2>&1",
            "xpu-smi discovery; printf '\n--- health ---\n'; xpu-smi health -l 2>/dev/null || true",
            80,
        ),
        command(
            "ipmi-health",
            "IPMI 机箱与传感器",
            "带外管理",
            "ipmi",
            "command -v ipmitool >/dev/null 2>&1",
            "run_ipmi() { if sudo -n true 2>/dev/null; then sudo -n ipmitool \"$@\"; else ipmitool \"$@\"; fi; }; run_ipmi mc info; printf '\n--- chassis ---\n'; run_ipmi chassis status; printf '\n--- sensors ---\n'; run_ipmi sensor; printf '\n--- recent sel ---\n'; run_ipmi sel elist last 24",
            90,
        ),
        command(
            "block-devices",
            "磁盘、文件系统与挂载",
            "存储",
            "generic",
            "command -v lsblk >/dev/null 2>&1",
            "lsblk -o NAME,PATH,TYPE,SIZE,MODEL,SERIAL,ROTA,TRAN,MOUNTPOINTS,FSTYPE,FSAVAIL,FSUSE%,STATE; printf '\n--- filesystems ---\n'; df -hT",
            100,
        ),
        command(
            "smart-health",
            "SMART 磁盘健康",
            "存储",
            "smart",
            "command -v smartctl >/dev/null 2>&1 && command -v lsblk >/dev/null 2>&1",
            "for dev in $(lsblk -ndo PATH,TYPE | awk '$2==\"disk\" {print $1}'); do printf '\n--- %s ---\n' \"$dev\"; sudo -n smartctl -H -A \"$dev\" 2>/dev/null || smartctl -H -A \"$dev\" 2>&1 || true; done",
            110,
        ),
        command(
            "nvme-health",
            "NVMe 设备健康",
            "存储",
            "nvme",
            "command -v nvme >/dev/null 2>&1",
            "nvme list; for dev in /dev/nvme*n1; do test -e \"$dev\" || continue; printf '\n--- %s ---\n' \"$dev\"; sudo -n nvme smart-log \"$dev\" 2>/dev/null || nvme smart-log \"$dev\" 2>&1 || true; done",
            120,
        ),
        command(
            "network-links",
            "网络接口与链路",
            "网络",
            "generic",
            "command -v ip >/dev/null 2>&1",
            "ip -br link; printf '\n--- addresses ---\n'; ip -br addr; printf '\n--- routes ---\n'; ip route",
            130,
        ),
        command(
            "failed-services",
            "systemd 异常服务",
            "服务",
            "systemd",
            "command -v systemctl >/dev/null 2>&1",
            "systemctl --failed --no-pager; printf '\n--- degraded ---\n'; systemctl is-system-running 2>&1 || true",
            140,
        ),
        command(
            "container-runtime",
            "容器运行状态",
            "服务",
            "container",
            "command -v docker >/dev/null 2>&1 || command -v podman >/dev/null 2>&1",
            "if command -v docker >/dev/null 2>&1; then docker ps --format 'table {{.Names}}\\t{{.Image}}\\t{{.Status}}\\t{{.Ports}}'; docker info --format 'Driver={{.Driver}} Containers={{.Containers}} Running={{.ContainersRunning}}' 2>/dev/null || true; else podman ps; fi",
            150,
        ),
    ]
}

fn command(
    code: &str,
    name: &str,
    category: &str,
    hardware_family: &str,
    detect_script: &str,
    command_script: &str,
    order_index: i64,
) -> UpsertSshCommandRequest {
    UpsertSshCommandRequest {
        code: code.to_string(),
        name: name.to_string(),
        category: category.to_string(),
        hardware_family: hardware_family.to_string(),
        detect_script: detect_script.to_string(),
        command_script: command_script.to_string(),
        kind: COMMAND_KIND_MONITOR.to_string(),
        timeout_secs: 15,
        enabled: true,
        order_index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_covers_major_accelerator_vendors() {
        let commands = builtin_commands();
        let families = commands
            .iter()
            .map(|command| command.hardware_family.as_str())
            .collect::<Vec<_>>();

        assert!(families.contains(&"hygon_dcu"));
        assert!(families.contains(&"nvidia_gpu"));
        assert!(families.contains(&"amd_rocm"));
        assert!(families.contains(&"intel_xpu"));
        assert!(
            commands
                .iter()
                .any(|command| command.command_script.contains("hy-smi"))
        );
    }
}
