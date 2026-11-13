//! What this machine offers games: its GPUs (including a laptop's hybrid pair), kernel
//! synchronisation primitives and helper programs. The launch options are chosen from this, so
//! the interface only ever offers what actually exists.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
    Intel,
    Amd,
    Nvidia,
    Other,
}

impl GpuVendor {
    fn from_id(id: u16) -> Self {
        match id {
            0x8086 => GpuVendor::Intel,
            0x1002 => GpuVendor::Amd,
            0x10de => GpuVendor::Nvidia,
            _ => GpuVendor::Other,
        }
    }

    fn label(self) -> &'static str {
        match self {
            GpuVendor::Intel => "Intel",
            GpuVendor::Amd => "AMD",
            GpuVendor::Nvidia => "NVIDIA",
            GpuVendor::Other => "GPU",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    /// PCI address, e.g. `0000:01:00.0`: the stable name of this GPU.
    pub id: String,
    pub vendor: GpuVendor,
    pub vendor_id: u16,
    pub device_id: u16,
    pub name: String,
    /// The GPU the desktop is displayed on. On a laptop this is usually the integrated one.
    pub primary: bool,
    pub driver: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hardware {
    pub gpus: Vec<Gpu>,
    /// `/dev/ntsync` exists: the kernel can run Wine's NT synchronisation natively.
    pub ntsync: bool,
    pub gamescope: Option<PathBuf>,
}

impl Hardware {
    /// More than one GPU that can render games (integrated + discrete).
    pub fn is_hybrid(&self) -> bool {
        self.gpus.len() > 1
    }

    /// The best GPU for games: a discrete one when there is one, else the only one.
    pub fn preferred_gpu(&self) -> Option<&Gpu> {
        self.gpus
            .iter()
            .find(|gpu| gpu.vendor == GpuVendor::Nvidia)
            .or_else(|| {
                self.gpus
                    .iter()
                    .find(|gpu| gpu.vendor == GpuVendor::Amd && !gpu.primary)
            })
            .or_else(|| self.gpus.iter().find(|gpu| gpu.vendor == GpuVendor::Amd))
            .or_else(|| self.gpus.first())
    }

    pub fn gpu(&self, id: &str) -> Option<&Gpu> {
        self.gpus.iter().find(|gpu| gpu.id == id)
    }
}

fn read_hex(path: &Path) -> Option<u16> {
    let text = fs::read_to_string(path).ok()?;
    u16::from_str_radix(text.trim().trim_start_matches("0x"), 16).ok()
}

/// Looks a device up in the system's PCI id list (`hwdata`), for a readable name.
fn pci_name(vendor: u16, device: u16) -> Option<String> {
    let list = ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"]
        .iter()
        .find_map(|path| fs::read_to_string(path).ok())?;
    parse_pci_name(&list, vendor, device)
}

fn parse_pci_name(list: &str, vendor: u16, device: u16) -> Option<String> {
    let vendor_line = format!("{vendor:04x}  ");
    let device_line = format!("\t{device:04x}  ");
    let mut in_vendor = false;
    for line in list.lines() {
        if !line.starts_with('\t') && !line.starts_with('#') && !line.is_empty() {
            in_vendor = line.starts_with(&vendor_line);
            continue;
        }
        if in_vendor && line.starts_with(&device_line) {
            return Some(line[device_line.len()..].trim().to_string());
        }
    }
    None
}

/// A tidy GPU name: the marketing name between the brackets when there is one
/// (`GP106M [GeForce GTX 1060 Mobile]` -> `GeForce GTX 1060 Mobile`).
fn tidy_name(raw: &str) -> String {
    match (raw.rfind('['), raw.rfind(']')) {
        (Some(start), Some(end)) if end > start => raw[start + 1..end].to_string(),
        _ => raw.to_string(),
    }
}

/// GPUs under `drm_root` (`/sys/class/drm`): one entry per `cardN`, ignoring display connectors.
pub fn detect_gpus(drm_root: &Path) -> Vec<Gpu> {
    let Ok(entries) = fs::read_dir(drm_root) else {
        return Vec::new();
    };
    let mut gpus: Vec<Gpu> = entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_prefix("card")
                .is_some_and(|rest| rest.chars().all(|c| c.is_ascii_digit()))
        })
        .filter_map(|entry| {
            let device = entry.path().join("device");
            let vendor_id = read_hex(&device.join("vendor"))?;
            let device_id = read_hex(&device.join("device"))?;
            let id = fs::canonicalize(&device)
                .ok()?
                .file_name()?
                .to_string_lossy()
                .into_owned();
            let vendor = GpuVendor::from_id(vendor_id);
            let name = pci_name(vendor_id, device_id)
                .map(|raw| tidy_name(&raw))
                .map(|name| format!("{} {name}", vendor.label()).replace("NVIDIA NVIDIA", "NVIDIA"))
                .map(|name| name.replace("NVIDIA Corporation ", "NVIDIA "))
                .unwrap_or_else(|| format!("{} GPU", vendor.label()));
            Some(Gpu {
                id,
                vendor,
                vendor_id,
                device_id,
                name,
                primary: fs::read_to_string(device.join("boot_vga"))
                    .is_ok_and(|text| text.trim() == "1"),
                driver: fs::read_link(device.join("driver")).ok().and_then(|link| {
                    link.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                }),
            })
        })
        .collect();
    gpus.sort_by(|a, b| a.id.cmp(&b.id));
    gpus
}

pub fn detect() -> Hardware {
    Hardware {
        gpus: detect_gpus(Path::new("/sys/class/drm")),
        ntsync: Path::new("/dev/ntsync").exists(),
        gamescope: crate::wine::find_in_path("gamescope"),
    }
}

/// Environment that makes a game render on `gpu`.
///
/// The desktop's own GPU needs nothing; the other one is reached through PRIME render offload
/// (NVIDIA's variables, Mesa's `DRI_PRIME`). On a machine with several GPUs, Vulkan is also
/// shown the chosen one only: DXVK and VKD3D turn every Vulkan device into a Direct3D adapter,
/// and a game that picks the integrated one renders badly or not at all (The Walking Dead: The
/// Telltale Definitive Series shows a white screen).
pub fn gpu_env(hardware: &Hardware, gpu: &Gpu) -> Vec<(String, String)> {
    let mut env = Vec::new();
    if !hardware.is_hybrid() {
        return env;
    }
    env.push((
        "MESA_VK_DEVICE_SELECT".into(),
        format!("{:04x}:{:04x}", gpu.vendor_id, gpu.device_id),
    ));
    env.push((
        "MESA_VK_DEVICE_SELECT_FORCE_DEFAULT_DEVICE".into(),
        "1".into(),
    ));

    let offload = !gpu.primary;
    match gpu.vendor {
        GpuVendor::Nvidia => {
            if offload {
                env.push(("__NV_PRIME_RENDER_OFFLOAD".into(), "1".into()));
                env.push(("__VK_LAYER_NV_optimus".into(), "NVIDIA_only".into()));
                env.push(("__GLX_VENDOR_LIBRARY_NAME".into(), "nvidia".into()));
            }
        }
        _ => {
            if offload {
                env.push((
                    "DRI_PRIME".into(),
                    format!("pci-{}", gpu.id.replace([':', '.'], "_")),
                ));
            }
            // With NVIDIA's driver around, OpenGL must be sent to Mesa explicitly.
            if hardware
                .gpus
                .iter()
                .any(|other| other.vendor == GpuVendor::Nvidia)
            {
                env.push(("__GLX_VENDOR_LIBRARY_NAME".into(), "mesa".into()));
            }
        }
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gpu(vendor: GpuVendor, id: &str, primary: bool) -> Gpu {
        let vendor_id = match vendor {
            GpuVendor::Intel => 0x8086,
            GpuVendor::Amd => 0x1002,
            GpuVendor::Nvidia => 0x10de,
            GpuVendor::Other => 0,
        };
        Gpu {
            id: id.into(),
            vendor,
            vendor_id,
            device_id: 0x1c20,
            name: "x".into(),
            primary,
            driver: None,
        }
    }

    fn laptop() -> Hardware {
        Hardware {
            gpus: vec![
                gpu(GpuVendor::Intel, "0000:00:02.0", true),
                gpu(GpuVendor::Nvidia, "0000:01:00.0", false),
            ],
            ..Hardware::default()
        }
    }

    #[test]
    fn the_discrete_gpu_is_preferred_on_a_hybrid_laptop() {
        let hardware = laptop();
        assert!(hardware.is_hybrid());
        assert_eq!(hardware.preferred_gpu().unwrap().vendor, GpuVendor::Nvidia);
    }

    #[test]
    fn nvidia_offload_uses_the_prime_variables_only_when_it_is_not_the_desktop_gpu() {
        let hardware = laptop();
        let env = gpu_env(&hardware, &hardware.gpus[1]);
        assert!(env.contains(&("__NV_PRIME_RENDER_OFFLOAD".into(), "1".into())));
        assert!(env.contains(&("__GLX_VENDOR_LIBRARY_NAME".into(), "nvidia".into())));
        // Direct3D games must not see the integrated GPU as a second adapter.
        assert!(env.contains(&("MESA_VK_DEVICE_SELECT".into(), "10de:1c20".into())));
        assert!(env.contains(&(
            "MESA_VK_DEVICE_SELECT_FORCE_DEFAULT_DEVICE".into(),
            "1".into()
        )));

        let desktop = Hardware {
            gpus: vec![gpu(GpuVendor::Nvidia, "0000:01:00.0", true)],
            ..Hardware::default()
        };
        assert!(gpu_env(&desktop, &desktop.gpus[0]).is_empty());
    }

    #[test]
    fn picking_the_integrated_gpu_next_to_nvidia_sends_opengl_to_mesa() {
        let hardware = laptop();
        let env = gpu_env(&hardware, &hardware.gpus[0]);
        assert!(env.contains(&("__GLX_VENDOR_LIBRARY_NAME".into(), "mesa".into())));
        assert!(env.iter().any(|(key, _)| key == "MESA_VK_DEVICE_SELECT"));
        assert!(!env.iter().any(|(key, _)| key == "DRI_PRIME"));
    }

    #[test]
    fn a_secondary_amd_gpu_is_selected_with_dri_prime() {
        let hardware = Hardware {
            gpus: vec![
                gpu(GpuVendor::Intel, "0000:00:02.0", true),
                gpu(GpuVendor::Amd, "0000:03:00.0", false),
            ],
            ..Hardware::default()
        };
        let env = gpu_env(&hardware, &hardware.gpus[1]);
        assert!(env.contains(&("DRI_PRIME".into(), "pci-0000_03_00_0".into())));
    }

    #[test]
    fn a_single_gpu_needs_no_selection() {
        let hardware = Hardware {
            gpus: vec![gpu(GpuVendor::Amd, "0000:03:00.0", true)],
            ..Hardware::default()
        };
        assert!(!hardware.is_hybrid());
        assert!(gpu_env(&hardware, &hardware.gpus[0]).is_empty());
    }

    #[test]
    fn pci_names_are_read_from_the_id_list_and_tidied() {
        let list = "# comment\n10de  NVIDIA Corporation\n\t1c20  GP106M [GeForce GTX 1060 Mobile]\n\t1c21  Other\n8086  Intel Corporation\n\t3e9b  CoffeeLake-H GT2 [UHD Graphics 630]\n";
        assert_eq!(
            parse_pci_name(list, 0x10de, 0x1c20).as_deref(),
            Some("GP106M [GeForce GTX 1060 Mobile]")
        );
        assert_eq!(parse_pci_name(list, 0x10de, 0x3e9b), None);
        assert_eq!(
            tidy_name("GP106M [GeForce GTX 1060 Mobile]"),
            "GeForce GTX 1060 Mobile"
        );
        assert_eq!(tidy_name("Plain"), "Plain");
    }
}
