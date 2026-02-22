//! Package Manager Abstraction Module
//!
//! Provides a unified interface for different package managers
//! (apt-get, pacman, dnf) with package name mapping.

use super::distro::Distro;

/// Package manager trait for cross-distro package operations
pub trait PackageManager {
    /// Returns the update command (e.g., "apt-get update")
    fn update_command(&self) -> String;

    /// Returns the install command for the given packages
    fn install_command(&self, packages: &[&str]) -> String;

    /// Returns the command to check if packages are installed
    fn check_command(&self, packages: &[&str]) -> String;

    /// Returns the package manager name for display
    fn name(&self) -> &'static str;

    /// Returns true if this package manager needs an update before install
    fn needs_update_before_install(&self) -> bool {
        true
    }
}

/// Debian/Ubuntu package manager (apt-get)
pub struct AptPackageManager;

impl PackageManager for AptPackageManager {
    fn update_command(&self) -> String {
        "sudo apt-get update".to_string()
    }

    fn install_command(&self, packages: &[&str]) -> String {
        format!("sudo apt-get install -y {}", packages.join(" "))
    }

    fn check_command(&self, packages: &[&str]) -> String {
        format!("dpkg -l {} 2>/dev/null", packages.join(" "))
    }

    fn name(&self) -> &'static str {
        "apt-get"
    }

    fn needs_update_before_install(&self) -> bool {
        true
    }
}

/// Arch Linux package manager (pacman)
pub struct PacmanPackageManager;

impl PackageManager for PacmanPackageManager {
    fn update_command(&self) -> String {
        "sudo pacman -Sy".to_string()
    }

    fn install_command(&self, packages: &[&str]) -> String {
        format!("sudo pacman -S --noconfirm --needed {}", packages.join(" "))
    }

    fn check_command(&self, packages: &[&str]) -> String {
        format!("pacman -Q {} 2>/dev/null", packages.join(" "))
    }

    fn name(&self) -> &'static str {
        "pacman"
    }

    fn needs_update_before_install(&self) -> bool {
        false // pacman -Sy does update in install command
    }
}

/// Fedora/RHEL package manager (dnf)
pub struct DnfPackageManager;

impl PackageManager for DnfPackageManager {
    fn update_command(&self) -> String {
        "sudo dnf check-update".to_string()
    }

    fn install_command(&self, packages: &[&str]) -> String {
        format!("sudo dnf install -y {}", packages.join(" "))
    }

    fn check_command(&self, packages: &[&str]) -> String {
        format!("rpm -q {} 2>/dev/null", packages.join(" "))
    }

    fn name(&self) -> &'static str {
        "dnf"
    }

    fn needs_update_before_install(&self) -> bool {
        false // dnf automatically refreshes
    }
}

/// macOS package manager (brew)
pub struct BrewPackageManager;

impl PackageManager for BrewPackageManager {
    fn update_command(&self) -> String {
        "brew update".to_string()
    }

    fn install_command(&self, packages: &[&str]) -> String {
        format!("brew install {}", packages.join(" "))
    }

    fn check_command(&self, packages: &[&str]) -> String {
        format!("brew list {} 2>/dev/null", packages.join(" "))
    }

    fn name(&self) -> &'static str {
        "brew"
    }
}

/// Generic/unknown package manager (fallback)
pub struct GenericPackageManager;

impl PackageManager for GenericPackageManager {
    fn update_command(&self) -> String {
        "# No update command for unknown distro".to_string()
    }

    fn install_command(&self, packages: &[&str]) -> String {
        // Try to use common package names and hope for the best
        format!("# Please install: {}", packages.join(" "))
    }

    fn check_command(&self, _packages: &[&str]) -> String {
        "false".to_string()
    }

    fn name(&self) -> &'static str {
        "unknown"
    }

    fn needs_update_before_install(&self) -> bool {
        false
    }
}

/// Returns the appropriate package manager for the given distribution
pub fn get_package_manager(distro: Distro) -> Box<dyn PackageManager> {
    match distro {
        Distro::Debian => Box::new(AptPackageManager),
        Distro::Arch => Box::new(PacmanPackageManager),
        Distro::Fedora => Box::new(DnfPackageManager),
        Distro::Macos => Box::new(BrewPackageManager),
        Distro::Unknown => Box::new(GenericPackageManager),
    }
}

/// Package purpose identifiers for cross-distro package mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackagePurpose {
    /// Basic build tools (build-essential, base-devel, @development-tools)
    BuildTools,
    /// curl
    Curl,
    /// unzip
    Unzip,
    /// pkg-config/pkgconf
    PkgConfig,
    /// OpenSSL development headers
    OpenSSL,
    /// ncurses development headers
    Ncurses,
    /// libevent development headers
    LibEvent,
    /// tmux terminal multiplexer
    Tmux,
    /// Go programming language
    Go,
    /// Universal ctags
    Ctags,
    /// Yazi file manager
    Yazi,
}

/// Returns the package name for a given purpose and distribution
pub fn get_package_name(purpose: PackagePurpose, distro: Distro) -> Option<&'static str> {
    match (purpose, distro) {
        // Build Tools
        (PackagePurpose::BuildTools, Distro::Debian) => Some("build-essential"),
        (PackagePurpose::BuildTools, Distro::Arch) => Some("base-devel"),
        (PackagePurpose::BuildTools, Distro::Fedora) => None, // Uses group install

        // curl (same on all)
        (PackagePurpose::Curl, Distro::Debian) => Some("curl"),
        (PackagePurpose::Curl, Distro::Arch) => Some("curl"),
        (PackagePurpose::Curl, Distro::Fedora) => Some("curl"),

        // unzip (same on all)
        (PackagePurpose::Unzip, Distro::Debian) => Some("unzip"),
        (PackagePurpose::Unzip, Distro::Arch) => Some("unzip"),
        (PackagePurpose::Unzip, Distro::Fedora) => Some("unzip"),

        // pkg-config
        (PackagePurpose::PkgConfig, Distro::Debian) => Some("pkg-config"),
        (PackagePurpose::PkgConfig, Distro::Arch) => Some("pkgconf"),
        (PackagePurpose::PkgConfig, Distro::Fedora) => Some("pkgconfig"),

        // OpenSSL
        (PackagePurpose::OpenSSL, Distro::Debian) => Some("libssl-dev"),
        (PackagePurpose::OpenSSL, Distro::Arch) => Some("openssl"),
        (PackagePurpose::OpenSSL, Distro::Fedora) => Some("openssl-devel"),

        // ncurses
        (PackagePurpose::Ncurses, Distro::Debian) => Some("libncurses-dev"),
        (PackagePurpose::Ncurses, Distro::Arch) => Some("ncurses"),
        (PackagePurpose::Ncurses, Distro::Fedora) => Some("ncurses-devel"),

        // libevent
        (PackagePurpose::LibEvent, Distro::Debian) => Some("libevent-dev"),
        (PackagePurpose::LibEvent, Distro::Arch) => Some("libevent"),
        (PackagePurpose::LibEvent, Distro::Fedora) => Some("libevent-devel"),

        // tmux (same on all)
        (PackagePurpose::Tmux, Distro::Debian) => Some("tmux"),
        (PackagePurpose::Tmux, Distro::Arch) => Some("tmux"),
        (PackagePurpose::Tmux, Distro::Fedora) => Some("tmux"),

        // Go
        (PackagePurpose::Go, Distro::Debian) => Some("golang-go"),
        (PackagePurpose::Go, Distro::Arch) => Some("go"),
        (PackagePurpose::Go, Distro::Fedora) => Some("golang"),

        // ctags
        (PackagePurpose::Ctags, Distro::Debian) => Some("universal-ctags"),
        (PackagePurpose::Ctags, Distro::Arch) => Some("ctags"),
        (PackagePurpose::Ctags, Distro::Fedora) => Some("ctags"),

        // yazi (same on all, may need cargo fallback)
        (PackagePurpose::Yazi, Distro::Debian) => Some("yazi"),
        (PackagePurpose::Yazi, Distro::Arch) => Some("yazi"),
        (PackagePurpose::Yazi, Distro::Fedora) => Some("yazi"),

        // Unknown distro - return None
        (_, Distro::Unknown) => None,
        (_, Distro::Macos) => None, // Handle macOS separately if needed
    }
}

/// Returns the package names for multiple purposes as a vector
pub fn get_package_names(purposes: &[PackagePurpose], distro: Distro) -> Vec<String> {
    purposes
        .iter()
        .filter_map(|&p| get_package_name(p, distro).map(|s| s.to_string()))
        .collect()
}

/// Returns the build tools installation command for the distribution
pub fn get_build_tools_install_command(distro: Distro) -> String {
    match distro {
        Distro::Fedora => "sudo dnf group install -y \"Development Tools\"".to_string(),
        Distro::Debian => {
            "sudo apt-get update && sudo apt-get install -y build-essential".to_string()
        }
        Distro::Arch => "sudo pacman -S --noconfirm --needed base-devel".to_string(),
        _ => "# Please install build tools manually".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apt_package_manager() {
        let pm = AptPackageManager;
        assert_eq!(pm.update_command(), "sudo apt-get update");
        assert_eq!(
            pm.install_command(&["curl", "wget"]),
            "sudo apt-get install -y curl wget"
        );
        assert_eq!(pm.name(), "apt-get");
        assert!(pm.needs_update_before_install());
    }

    #[test]
    fn test_pacman_package_manager() {
        let pm = PacmanPackageManager;
        assert_eq!(pm.update_command(), "sudo pacman -Sy");
        assert_eq!(
            pm.install_command(&["curl", "wget"]),
            "sudo pacman -S --noconfirm --needed curl wget"
        );
        assert_eq!(pm.name(), "pacman");
        assert!(!pm.needs_update_before_install());
    }

    #[test]
    fn test_dnf_package_manager() {
        let pm = DnfPackageManager;
        assert_eq!(pm.update_command(), "sudo dnf check-update");
        assert_eq!(
            pm.install_command(&["curl", "wget"]),
            "sudo dnf install -y curl wget"
        );
        assert_eq!(pm.name(), "dnf");
        assert!(!pm.needs_update_before_install());
    }

    #[test]
    fn test_get_package_manager() {
        assert_eq!(get_package_manager(Distro::Debian).name(), "apt-get");
        assert_eq!(get_package_manager(Distro::Arch).name(), "pacman");
        assert_eq!(get_package_manager(Distro::Fedora).name(), "dnf");
        assert_eq!(get_package_manager(Distro::Unknown).name(), "unknown");
    }

    #[test]
    fn test_package_name_mapping_debian() {
        assert_eq!(
            get_package_name(PackagePurpose::BuildTools, Distro::Debian),
            Some("build-essential")
        );
        assert_eq!(
            get_package_name(PackagePurpose::PkgConfig, Distro::Debian),
            Some("pkg-config")
        );
        assert_eq!(
            get_package_name(PackagePurpose::OpenSSL, Distro::Debian),
            Some("libssl-dev")
        );
        assert_eq!(
            get_package_name(PackagePurpose::Go, Distro::Debian),
            Some("golang-go")
        );
    }

    #[test]
    fn test_package_name_mapping_arch() {
        assert_eq!(
            get_package_name(PackagePurpose::BuildTools, Distro::Arch),
            Some("base-devel")
        );
        assert_eq!(
            get_package_name(PackagePurpose::PkgConfig, Distro::Arch),
            Some("pkgconf")
        );
        assert_eq!(
            get_package_name(PackagePurpose::OpenSSL, Distro::Arch),
            Some("openssl")
        );
        assert_eq!(
            get_package_name(PackagePurpose::Go, Distro::Arch),
            Some("go")
        );
    }

    #[test]
    fn test_package_name_mapping_fedora() {
        assert_eq!(
            get_package_name(PackagePurpose::BuildTools, Distro::Fedora),
            None
        ); // Uses group
        assert_eq!(
            get_package_name(PackagePurpose::PkgConfig, Distro::Fedora),
            Some("pkgconfig")
        );
        assert_eq!(
            get_package_name(PackagePurpose::OpenSSL, Distro::Fedora),
            Some("openssl-devel")
        );
        assert_eq!(
            get_package_name(PackagePurpose::Go, Distro::Fedora),
            Some("golang")
        );
    }

    #[test]
    fn test_get_package_names() {
        let purposes = vec![
            PackagePurpose::Curl,
            PackagePurpose::Unzip,
            PackagePurpose::PkgConfig,
        ];

        let debian_pkgs = get_package_names(&purposes, Distro::Debian);
        assert_eq!(debian_pkgs, vec!["curl", "unzip", "pkg-config"]);

        let arch_pkgs = get_package_names(&purposes, Distro::Arch);
        assert_eq!(arch_pkgs, vec!["curl", "unzip", "pkgconf"]);

        let fedora_pkgs = get_package_names(&purposes, Distro::Fedora);
        assert_eq!(fedora_pkgs, vec!["curl", "unzip", "pkgconfig"]);
    }

    #[test]
    fn test_build_tools_install_command() {
        assert!(get_build_tools_install_command(Distro::Debian).contains("apt-get"));
        assert!(get_build_tools_install_command(Distro::Arch).contains("pacman"));
        assert!(get_build_tools_install_command(Distro::Fedora).contains("dnf group"));
    }
}
