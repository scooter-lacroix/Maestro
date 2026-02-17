//! Linux Distribution Detection Module
//!
//! Detects the current Linux distribution using /etc/os-release
//! with fallback to lsb_release command.

use std::collections::HashMap;
use std::fs;
use std::process::Command;

/// Supported Linux distributions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Distro {
    /// Debian and derivatives (Ubuntu, Linux Mint, Pop!_OS, etc.)
    Debian,
    /// Arch and derivatives (Arch Linux, CachyOS, Manjaro, EndeavourOS, etc.)
    Arch,
    /// Fedora and derivatives (Fedora, RHEL, CentOS, AlmaLinux, Rocky Linux, etc.)
    Fedora,
    /// macOS (Darwin)
    Macos,
    /// Unknown or unsupported distribution
    Unknown,
}

impl Distro {
    /// Returns a human-readable name for the distribution
    pub fn display_name(&self) -> &'static str {
        match self {
            Distro::Debian => "Debian/Ubuntu",
            Distro::Arch => "Arch Linux",
            Distro::Fedora => "Fedora/RHEL",
            Distro::Macos => "macOS",
            Distro::Unknown => "Unknown",
        }
    }

    /// Returns the package manager command name for display
    pub fn package_manager_name(&self) -> &'static str {
        match self {
            Distro::Debian => "apt-get",
            Distro::Arch => "pacman",
            Distro::Fedora => "dnf",
            Distro::Macos => "brew",
            Distro::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for Distro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Detects the current Linux distribution
///
/// Detection method:
/// 1. Read /etc/os-release (POSIX standard since 2013)
/// 2. Parse ID and ID_LIKE fields
/// 3. Fallback to lsb_release command if /etc/os-release not found
///
/// # Returns
///
/// The detected distribution, or Distro::Unknown if detection fails
pub fn detect_distro() -> Distro {
    // Try /etc/os-release first (standard location)
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        if let Some(distro) = parse_os_release(&content) {
            return distro;
        }
    }

    // Fallback: Try lsb_release command
    if let Some(distro) = detect_via_lsb_release() {
        return distro;
    }

    // Check for macOS
    if cfg!(target_os = "macos") {
        return Distro::Macos;
    }

    Distro::Unknown
}

/// Parses /etc/os-release content to detect distribution
fn parse_os_release(content: &str) -> Option<Distro> {
    let vars = parse_os_release_vars(content);
    
    let id = vars.get("ID").map(|s| s.to_lowercase());
    let id_like = vars.get("ID_LIKE").map(|s| s.to_lowercase());

    match id.as_deref() {
        // Arch and derivatives
        Some("arch") => Some(Distro::Arch),
        Some("cachyos") => Some(Distro::Arch),
        Some("manjaro") => Some(Distro::Arch),
        Some("endeavouros") => Some(Distro::Arch),
        Some("arcolinux") => Some(Distro::Arch),
        Some("garuda") => Some(Distro::Arch),
        
        // Fedora and derivatives
        Some("fedora") => Some(Distro::Fedora),
        Some("rhel") => Some(Distro::Fedora),
        Some("centos") => Some(Distro::Fedora),
        Some("almalinux") => Some(Distro::Fedora),
        Some("rocky") | Some("rockylinux") => Some(Distro::Fedora),
        
        // Debian and derivatives
        Some("debian") => Some(Distro::Debian),
        Some("ubuntu") => Some(Distro::Debian),
        Some("linuxmint") | Some("linux mint") => Some(Distro::Debian),
        Some("pop") | Some("pop_os") => Some(Distro::Debian),
        Some("elementary") | Some("elementaryos") => Some(Distro::Debian),
        Some("kali") => Some(Distro::Debian),
        Some("raspbian") => Some(Distro::Debian),
        Some("zorin") => Some(Distro::Debian),
        
        // Check ID_LIKE for derivative detection
        _ => {
            if let Some(like) = id_like.as_deref() {
                // ID_LIKE can contain multiple space-separated values
                if like.contains("arch") {
                    return Some(Distro::Arch);
                }
                if like.contains("fedora") || like.contains("rhel") || like.contains("centos") {
                    return Some(Distro::Fedora);
                }
                if like.contains("debian") || like.contains("ubuntu") {
                    return Some(Distro::Debian);
                }
            }
            None
        }
    }
}

/// Parses /etc/os-release into a HashMap of key-value pairs
fn parse_os_release_vars(content: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            // Remove quotes from value if present
            let value = value.trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            vars.insert(key, value);
        }
    }
    
    vars
}

/// Fallback detection using lsb_release command
fn detect_via_lsb_release() -> Option<Distro> {
    let output = Command::new("lsb_release")
        .arg("-i")
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    
    if stdout.contains("arch") || stdout.contains("manjaro") || stdout.contains("cachyos") {
        return Some(Distro::Arch);
    }
    if stdout.contains("fedora") || stdout.contains("red hat") || stdout.contains("centos") {
        return Some(Distro::Fedora);
    }
    if stdout.contains("debian") || stdout.contains("ubuntu") || stdout.contains("mint") {
        return Some(Distro::Debian);
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_arch() {
        let content = r#"
ID=arch
NAME="Arch Linux"
VERSION_ID=20240101
"#;
        assert_eq!(parse_os_release(content), Some(Distro::Arch));
    }

    #[test]
    fn test_detect_cachyos() {
        let content = r#"
ID=cachyos
NAME="CachyOS"
VERSION=240101
"#;
        assert_eq!(parse_os_release(content), Some(Distro::Arch));
    }

    #[test]
    fn test_detect_manjaro() {
        let content = r#"
ID=manjaro
NAME="Manjaro Linux"
VERSION_ID=23.1.0
"#;
        assert_eq!(parse_os_release(content), Some(Distro::Arch));
    }

    #[test]
    fn test_detect_fedora() {
        let content = r#"
ID=fedora
NAME="Fedora Linux"
VERSION_ID=39
"#;
        assert_eq!(parse_os_release(content), Some(Distro::Fedora));
    }

    #[test]
    fn test_detect_ubuntu() {
        let content = r#"
ID=ubuntu
NAME="Ubuntu"
VERSION_ID="24.04"
"#;
        assert_eq!(parse_os_release(content), Some(Distro::Debian));
    }

    #[test]
    fn test_detect_debian() {
        let content = r#"
ID=debian
NAME="Debian GNU/Linux"
VERSION_ID="12"
"#;
        assert_eq!(parse_os_release(content), Some(Distro::Debian));
    }

    #[test]
    fn test_detect_debian_derivative_via_id_like() {
        let content = r#"
ID=pop
NAME="Pop!_OS"
VERSION_ID="22.04"
ID_LIKE="ubuntu debian"
"#;
        assert_eq!(parse_os_release(content), Some(Distro::Debian));
    }

    #[test]
    fn test_detect_arch_derivative_via_id_like() {
        let content = r#"
ID=somearch
NAME="Some Arch Derivative"
ID_LIKE="arch"
"#;
        assert_eq!(parse_os_release(content), Some(Distro::Arch));
    }

    #[test]
    fn test_detect_unknown() {
        let content = r#"
ID=gentoo
NAME="Gentoo Linux"
"#;
        assert_eq!(parse_os_release(content), None);
    }

    #[test]
    fn test_parse_os_release_vars() {
        let content = r#"
ID="ubuntu"
NAME='Ubuntu 24.04 LTS'
VERSION_ID="24.04"
# Comment line
ID_LIKE=debian
"#;
        let vars = parse_os_release_vars(content);
        assert_eq!(vars.get("ID"), Some(&"ubuntu".to_string()));
        assert_eq!(vars.get("NAME"), Some(&"Ubuntu 24.04 LTS".to_string()));
        assert_eq!(vars.get("VERSION_ID"), Some(&"24.04".to_string()));
        assert_eq!(vars.get("ID_LIKE"), Some(&"debian".to_string()));
    }

    #[test]
    fn test_distro_display() {
        assert_eq!(Distro::Arch.to_string(), "Arch Linux");
        assert_eq!(Distro::Debian.to_string(), "Debian/Ubuntu");
        assert_eq!(Distro::Fedora.to_string(), "Fedora/RHEL");
        assert_eq!(Distro::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_package_manager_name() {
        assert_eq!(Distro::Arch.package_manager_name(), "pacman");
        assert_eq!(Distro::Debian.package_manager_name(), "apt-get");
        assert_eq!(Distro::Fedora.package_manager_name(), "dnf");
    }

    #[test]
    fn test_malformed_os_release() {
        let content = "not valid os-release content";
        let vars = parse_os_release_vars(content);
        assert!(vars.is_empty());
    }

    #[test]
    fn test_empty_os_release() {
        let content = "";
        assert_eq!(parse_os_release(content), None);
    }
}
