/// Android hardware tier detection and performance guardrails.
///
/// Classifies devices into Low, Mid, and High tiers based on total physical RAM
/// (via `/proc/meminfo`) and GPU capabilities (`wgpu::AdapterInfo`), offering
/// safe recommended render distances while preserving player configuration freedom.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceTier {
    /// Low Tier: devices with <= 4 GB physical RAM. Safe default: 12 chunks.
    Low,
    /// Mid Tier: devices with 6–8 GB physical RAM. Safe default: 24 chunks.
    Mid,
    /// High Tier: devices with > 8 GB physical RAM and flagship GPU. Safe default: 48 chunks.
    High,
}

impl DeviceTier {
    /// Safe recommended render distance in chunks for this device tier.
    pub const fn recommended_render_distance(self) -> u32 {
        match self {
            Self::Low => 12,
            Self::Mid => 24,
            Self::High => 48,
        }
    }

    /// Display name of the hardware tier.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Low => "Low Tier",
            Self::Mid => "Mid Tier",
            Self::High => "High Tier",
        }
    }

    /// Approximate physical RAM bracket for this tier.
    pub const fn ram_label(self) -> &'static str {
        match self {
            Self::Low => "≤ 4GB RAM",
            Self::Mid => "6–8GB RAM",
            Self::High => "> 8GB RAM",
        }
    }
}

/// Parses `MemTotal: <number> kB` from `/proc/meminfo` content.
pub fn parse_mem_total_kib(content: &str) -> Option<u64> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let mut parts = rest.split_whitespace();
            if let Some(num_str) = parts.next() {
                if let Ok(kib) = num_str.parse::<u64>() {
                    return Some(kib);
                }
            }
        }
    }
    None
}

/// Reads `/proc/meminfo` directly on Android/Linux kernels.
pub fn read_mem_total_kib() -> Option<u64> {
    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        let content = std::fs::read_to_string("/proc/meminfo").ok()?;
        parse_mem_total_kib(&content)
    }
    #[cfg(not(any(target_os = "android", target_os = "linux")))]
    {
        None
    }
}

/// Checks if a GPU name and limits qualify for the High Tier bracket.
pub fn is_capable_high_end_gpu(gpu_name: &str, max_texture_dimension_2d: u32) -> bool {
    if max_texture_dimension_2d < 8192 {
        return false;
    }

    let name = gpu_name.to_ascii_lowercase();

    // Qualcomm Adreno flagship series: 7xx (Snapdragon 8 Gen 1/2/3), X1-85 (Snapdragon 8 Elite),
    // and Snapdragon 865/870/888 (650, 660).
    if name.contains("adreno") {
        if name.contains("730")
            || name.contains("740")
            || name.contains("750")
            || name.contains("x1")
            || name.contains("650")
            || name.contains("660")
        {
            return true;
        }
        // Exclude budget/entry Adreno explicitly
        if name.contains("610")
            || name.contains("612")
            || name.contains("615")
            || name.contains("616")
            || name.contains("618")
            || name.contains("619")
            || name.contains("506")
        {
            return false;
        }
    }

    // ARM Immortalis flagship GPUs
    if name.contains("immortalis") {
        return true;
    }

    // ARM Mali flagship series: G7xx (G710, G715, G720, G725, G77, G78)
    if name.contains("mali") {
        if name.contains("g710")
            || name.contains("g715")
            || name.contains("g720")
            || name.contains("g725")
            || name.contains("g77")
            || name.contains("g78")
            || name.contains("g925")
        {
            return true;
        }
        // Exclude entry/mid Mali explicitly
        if name.contains("g52")
            || name.contains("g57")
            || name.contains("g68")
            || name.contains("g72")
            || name.contains("g76")
        {
            return false;
        }
    }

    // Samsung / AMD RDNA mobile GPUs
    if name.contains("xclipse") {
        return true;
    }

    // Turnip open-source Vulkan driver
    if name.contains("turnip") {
        return true;
    }

    // Apple Silicon / Desktop discrete GPU (in test/cross-platform builds)
    if name.contains("apple") || name.contains("nvidia") || name.contains("radeon") {
        return true;
    }

    false
}

/// Classifies device tier given available RAM in KiB and optional GPU characteristics.
pub fn classify_tier(ram_kib: Option<u64>, gpu: Option<(&str, u32)>) -> DeviceTier {
    let ram = match ram_kib {
        Some(k) => k,
        None => return DeviceTier::Mid, // Conservative default when memory info is unavailable
    };

    // Account for kernel/modem reserved memory below advertised capacity:
    // - 4GB device reports ~3.6GB-3.8GB (< 5,000,000 kB)
    // - 6GB/8GB device reports ~5.3GB-7.6GB (5,000,000 .. 9,500,000 kB)
    // - 12GB+ device reports >= 10.8GB (>= 9,500,000 kB)
    if ram < 5_000_000 {
        DeviceTier::Low
    } else if ram < 9_500_000 {
        DeviceTier::Mid
    } else {
        // High RAM: check GPU capability to prevent pairing flagship RAM with entry-level GPU
        if let Some((name, max_tex)) = gpu {
            if is_capable_high_end_gpu(name, max_tex) {
                DeviceTier::High
            } else {
                DeviceTier::Mid
            }
        } else {
            DeviceTier::High
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_meminfo_memtotal() {
        let sample = "\
MemTotal:        7891232 kB
MemFree:          523412 kB
MemAvailable:    3981240 kB
Buffers:          124912 kB
";
        assert_eq!(parse_mem_total_kib(sample), Some(7891232));
    }

    #[test]
    fn classifies_tiers_accurately() {
        // 4GB phone
        assert_eq!(classify_tier(Some(3782164), None), DeviceTier::Low);
        // 6GB phone
        assert_eq!(classify_tier(Some(5691232), None), DeviceTier::Mid);
        // 8GB phone
        assert_eq!(classify_tier(Some(7623192), None), DeviceTier::Mid);
        // 12GB phone with flagship GPU
        assert_eq!(
            classify_tier(Some(11342110), Some(("Adreno (TM) 740", 16384))),
            DeviceTier::High
        );
        // 12GB phone with entry-level GPU gets demoted to Mid
        assert_eq!(
            classify_tier(Some(11342110), Some(("Mali-G57 MC2", 4096))),
            DeviceTier::Mid
        );
    }

    #[test]
    fn gpu_detection_flags() {
        assert!(is_capable_high_end_gpu("Adreno (TM) 750", 16384));
        assert!(is_capable_high_end_gpu("Immortalis-G720", 16384));
        assert!(is_capable_high_end_gpu("Samsung Xclipse 940", 16384));
        assert!(!is_capable_high_end_gpu("Adreno (TM) 610", 8192));
        assert!(!is_capable_high_end_gpu("Mali-G52", 4096));
    }
}
