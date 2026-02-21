//! LSP Registry - Known Rust-based Language Servers
//!
//! This module contains the registry of LSP servers that can be installed
//! via cargo or other package managers. All listed LSPs are written in Rust
//! or have Rust-based implementations.
//!
//! ## Cross-Platform Support
//!
//! The installer supports multiple installation methods with distro-specific
//! system package fallbacks:
//! - Arch/CachyOS: Uses pacman when available, falls back to cargo
//! - Debian/Ubuntu: Uses apt when available, falls back to cargo
//! - Fedora: Uses dnf when available, falls back to cargo
//! - macOS: Uses brew when available, falls back to cargo

use leindex_core::setup::distro::{detect_distro, Distro};

/// Result of LSP installation containing output logs and status
#[derive(Clone, Debug)]
pub struct InstallResult {
    pub success: bool,
    pub output: Vec<String>,
    pub command_used: String,
}

#[derive(Clone, Debug)]
pub struct LspInfo {
    pub name: &'static str,
    pub display_name: &'static str,
    pub language: &'static str,
    pub file_extensions: &'static [&'static str],
    pub install_method: LspInstallMethod,
    pub binary_name: &'static str,
    pub lsp_command: &'static str,
    pub description: &'static str,
    pub homepage: &'static str,
}

/// Installation method for LSPs with cross-distro package support
#[derive(Clone, Debug)]
pub enum LspInstallMethod {
    /// Install via cargo (fallback for all systems)
    Cargo { crate_name: &'static str },
    /// Install via rustup component
    Rustup { component: &'static str },
    /// Install via pip (Python packages)
    Pip { package: &'static str },
    /// Install via npm (Node.js packages)
    Npm { package: &'static str },
    /// Install via go install
    Go { package: &'static str },
    /// Custom installation command
    Custom {
        command: &'static str,
        notes: &'static str,
    },
    /// System package with distro-specific names
    /// Falls back to cargo if system package installation fails
    SystemPackage {
        /// Cargo crate name for fallback (empty string if not a crate)
        crate_name: &'static str,
        /// Package names for each distro (debian, arch, fedora, macos)
        debian_pkg: &'static str,
        arch_pkg: &'static str,
        fedora_pkg: &'static str,
        macos_pkg: &'static str,
    },
}

pub fn get_available_lsps() -> Vec<LspInfo> {
    vec![
        LspInfo {
            name: "rust-analyzer",
            display_name: "Rust Analyzer",
            language: "Rust",
            file_extensions: &["rs"],
            install_method: LspInstallMethod::Rustup {
                component: "rust-analyzer",
            },
            binary_name: "rust-analyzer",
            lsp_command: "rust-analyzer",
            description: "Official Rust language server with IDE features",
            homepage: "https://rust-analyzer.github.io/",
        },
        LspInfo {
            name: "ruff",
            display_name: "Ruff",
            language: "Python",
            file_extensions: &["py", "pyi", "pyw"],
            // ruff is available in system packages on most distros
            install_method: LspInstallMethod::SystemPackage {
                crate_name: "ruff",
                debian_pkg: "ruff", // Available in Ubuntu 24.04+
                arch_pkg: "ruff",   // Available in extra
                fedora_pkg: "ruff", // Available in Fedora 39+
                macos_pkg: "ruff",  // Available in homebrew
            },
            binary_name: "ruff",
            lsp_command: "ruff server",
            description: "Fast Python linter and formatter with LSP mode",
            homepage: "https://docs.astral.sh/ruff/",
        },
        LspInfo {
            name: "typescript-language-server",
            display_name: "TypeScript Language Server",
            language: "TypeScript/JavaScript",
            file_extensions: &["ts", "tsx", "js", "jsx", "mjs", "cjs"],
            // typescript-language-server available as system package
            install_method: LspInstallMethod::SystemPackage {
                crate_name: "typescript-language-server",
                debian_pkg: "typescript-language-server", // Ubuntu 24.04+
                arch_pkg: "typescript-language-server",   // extra
                fedora_pkg: "typescript-language-server", // Fedora 39+
                macos_pkg: "typescript-language-server",  // homebrew
            },
            binary_name: "typescript-language-server",
            lsp_command: "typescript-language-server --stdio",
            description: "TypeScript/JavaScript language server",
            homepage: "https://github.com/typescript-language-server/typescript-language-server",
        },
        LspInfo {
            name: "taplo",
            display_name: "Taplo",
            language: "TOML",
            file_extensions: &["toml"],
            install_method: LspInstallMethod::SystemPackage {
                crate_name: "taplo-cli",
                debian_pkg: "",     // Not available in Debian repos
                arch_pkg: "taplo",  // extra
                fedora_pkg: "",     // Not available in Fedora repos
                macos_pkg: "taplo", // homebrew
            },
            binary_name: "taplo",
            lsp_command: "taplo lsp stdio",
            description: "TOML toolkit with language server",
            homepage: "https://taplo.tamasfe.dev/",
        },
        LspInfo {
            name: "asm-lsp",
            display_name: "Assembly LSP",
            language: "Assembly",
            file_extensions: &["asm", "s", "S"],
            install_method: LspInstallMethod::Cargo {
                crate_name: "asm-lsp",
            },
            binary_name: "asm-lsp",
            lsp_command: "asm-lsp",
            description: "Assembly language server with x86/x86_64 support",
            homepage: "https://github.com/bergercookie/asm-lsp",
        },
        LspInfo {
            name: "gopls",
            display_name: "Go Language Server",
            language: "Go",
            file_extensions: &["go"],
            // gopls is available in system packages, with go install fallback
            install_method: LspInstallMethod::SystemPackage {
                crate_name: "", // Not a cargo crate
                debian_pkg: "gopls",
                arch_pkg: "gopls",
                fedora_pkg: "gopls",
                macos_pkg: "gopls",
            },
            binary_name: "gopls",
            lsp_command: "gopls",
            description: "Official Go language server",
            homepage: "https://go.dev/gopls/",
        },
        LspInfo {
            name: "bash-language-server",
            display_name: "Bash Language Server",
            language: "Bash/Shell",
            file_extensions: &["sh", "bash", "zsh"],
            install_method: LspInstallMethod::Npm {
                package: "bash-language-server",
            },
            binary_name: "bash-language-server",
            lsp_command: "bash-language-server start",
            description: "Bash language server with shellcheck integration",
            homepage: "https://github.com/bash-lsp/bash-language-server",
        },
        LspInfo {
            name: "yaml-language-server",
            display_name: "YAML Language Server",
            language: "YAML",
            file_extensions: &["yaml", "yml"],
            install_method: LspInstallMethod::Npm {
                package: "yaml-language-server",
            },
            binary_name: "yaml-language-server",
            lsp_command: "yaml-language-server --stdio",
            description: "YAML language server with schema support",
            homepage: "https://github.com/redhat-developer/yaml-language-server",
        },
        LspInfo {
            name: "json-lsp",
            display_name: "JSON Language Server",
            language: "JSON",
            file_extensions: &["json", "jsonc"],
            install_method: LspInstallMethod::Npm {
                package: "vscode-json-languageserver",
            },
            binary_name: "vscode-json-languageserver",
            lsp_command: "vscode-json-languageserver --stdio",
            description: "JSON language server with schema validation",
            homepage: "https://github.com/hrsh7th/vscode-langservers-extracted",
        },
        LspInfo {
            name: "markdown-oxide",
            display_name: "Markdown Oxide",
            language: "Markdown",
            file_extensions: &["md", "markdown"],
            install_method: LspInstallMethod::Cargo {
                crate_name: "markdown-oxide",
            },
            binary_name: "markdown-oxide",
            lsp_command: "markdown-oxide",
            description: "Rust-based Markdown language server",
            homepage: "https://github.com/Feel-ix-343/markdown-oxide",
        },
        LspInfo {
            name: "lexical-lsp",
            display_name: "Elixir LSP",
            language: "Elixir",
            file_extensions: &["ex", "exs", "eex", "heex", "leex"],
            install_method: LspInstallMethod::Custom {
                command: "mix install",
                notes: "Install via: mix escript.install hex lexical",
            },
            binary_name: "lexical",
            lsp_command: "lexical",
            description: "Elixir language server",
            homepage: "https://github.com/lexical-lsp/lexical",
        },
        LspInfo {
            name: "helm-ls",
            display_name: "Helm Language Server",
            language: "Helm",
            file_extensions: &["yaml", "tpl"],
            install_method: LspInstallMethod::Cargo {
                crate_name: "helm-ls",
            },
            binary_name: "helm-ls",
            lsp_command: "helm_ls",
            description: "Helm chart language server",
            homepage: "https://github.com/mrjosh/helm-ls",
        },
    ]
}

pub fn get_lsp_by_name(name: &str) -> Option<LspInfo> {
    get_available_lsps()
        .into_iter()
        .find(|lsp| lsp.name == name)
}

pub fn get_lsp_for_extension(ext: &str) -> Vec<LspInfo> {
    get_available_lsps()
        .into_iter()
        .filter(|lsp| lsp.file_extensions.contains(&ext))
        .collect()
}

/// Get the display command for the UI (shows what will be run)
pub fn get_install_command(lsp: &LspInfo) -> String {
    let distro = detect_distro();
    get_install_command_for_distro(lsp, distro)
}

/// Get the display command for a specific distro
pub fn get_install_command_for_distro(lsp: &LspInfo, distro: Distro) -> String {
    match &lsp.install_method {
        LspInstallMethod::Cargo { crate_name } => {
            format!("cargo install {}", crate_name)
        }
        LspInstallMethod::Rustup { component } => {
            format!("rustup component add {}", component)
        }
        LspInstallMethod::Pip { package } => {
            format!("pip install {}", package)
        }
        LspInstallMethod::Npm { package } => {
            format!("npm install -g {}", package)
        }
        LspInstallMethod::Go { package } => {
            format!("go install {}", package)
        }
        LspInstallMethod::Custom { command, notes } => {
            format!("{} # {}", command, notes)
        }
        LspInstallMethod::SystemPackage {
            crate_name,
            debian_pkg,
            arch_pkg,
            fedora_pkg,
            macos_pkg,
        } => {
            // Try system package first, show that in UI
            let (pkg_name, pkg_cmd) = match distro {
                Distro::Debian => (*debian_pkg, "sudo apt-get install -y"),
                Distro::Arch => (*arch_pkg, "sudo pacman -S --noconfirm --needed"),
                Distro::Fedora => (*fedora_pkg, "sudo dnf install -y"),
                Distro::Macos => (*macos_pkg, "brew install"),
                Distro::Unknown => ("", ""),
            };

            if !pkg_name.is_empty() {
                format!("{} {}", pkg_cmd, pkg_name)
            } else if !crate_name.is_empty() {
                format!(
                    "cargo install {}  # (fallback, no system package)",
                    crate_name
                )
            } else {
                "# No installation method available for this distro".to_string()
            }
        }
    }
}

/// Execute LSP installation with captured output
///
/// This function:
/// 1. Detects the current distro
/// 2. Tries system package manager first (for SystemPackage method)
/// 3. Falls back to cargo/npm/pip/go as appropriate
/// 4. Captures all output to prevent TUI corruption
/// 5. Returns structured result for display in UI
pub fn install_lsp(lsp: &LspInfo) -> InstallResult {
    let distro = detect_distro();
    install_lsp_for_distro(lsp, distro)
}

/// Install LSP for a specific distro
pub fn install_lsp_for_distro(lsp: &LspInfo, distro: Distro) -> InstallResult {
    let mut output = Vec::new();
    output.push(format!("Installing {} on {}...", lsp.display_name, distro));

    match &lsp.install_method {
        LspInstallMethod::Cargo { crate_name } => {
            let cmd = format!("cargo install {}", crate_name);
            output.push(format!("Running: {}", cmd));
            run_captured_command(&cmd, output, &format!("cargo install {}", crate_name))
        }
        LspInstallMethod::Rustup { component } => {
            let cmd = format!("rustup component add {}", component);
            output.push(format!("Running: {}", cmd));
            run_captured_command(&cmd, output, &cmd)
        }
        LspInstallMethod::Pip { package } => {
            let cmd = format!("pip install {}", package);
            output.push(format!("Running: {}", cmd));
            run_captured_command(&cmd, output, &cmd)
        }
        LspInstallMethod::Npm { package } => {
            let cmd = format!("npm install -g {}", package);
            output.push(format!("Running: {}", cmd));
            run_captured_command(&cmd, output, &cmd)
        }
        LspInstallMethod::Go { package } => {
            let cmd = format!("go install {}", package);
            output.push(format!("Running: {}", cmd));
            run_captured_command(&cmd, output, &cmd)
        }
        LspInstallMethod::Custom { command, notes } => {
            output.push(format!("Note: {}", notes));
            output.push(format!("Running: {}", command));
            run_captured_command(command, output, command)
        }
        LspInstallMethod::SystemPackage {
            crate_name,
            debian_pkg,
            arch_pkg,
            fedora_pkg,
            macos_pkg,
        } => {
            // Get the system package name for this distro
            let (pkg_name, pkg_cmd_prefix) = match distro {
                Distro::Debian => (*debian_pkg, "sudo apt-get install -y"),
                Distro::Arch => (*arch_pkg, "sudo pacman -S --noconfirm --needed"),
                Distro::Fedora => (*fedora_pkg, "sudo dnf install -y"),
                Distro::Macos => (*macos_pkg, "brew install"),
                Distro::Unknown => ("", ""),
            };

            // Try system package first if available
            if !pkg_name.is_empty() {
                let cmd = format!("{} {}", pkg_cmd_prefix, pkg_name);
                output.push(format!("Trying system package: {}", cmd));

                let result = run_captured_command(&cmd, output.clone(), &cmd);
                if result.success {
                    return result;
                }

                output = result.output;
                output.push("System package installation failed, trying fallback...".to_string());
            }

            // Fall back to cargo if available
            if !crate_name.is_empty() {
                let cmd = format!("cargo install {}", crate_name);
                output.push(format!("Trying cargo fallback: {}", cmd));
                run_captured_command(&cmd, output, &cmd)
            } else {
                // For gopls, fall back to go install
                if lsp.name == "gopls" {
                    let cmd = "go install golang.org/x/tools/gopls@latest".to_string();
                    output.push(format!("Trying go install: {}", cmd));
                    run_captured_command(&cmd, output, &cmd)
                } else {
                    output.push("ERROR: No fallback installation method available".to_string());
                    InstallResult {
                        success: false,
                        output,
                        command_used: "none".to_string(),
                    }
                }
            }
        }
    }
}

/// Run a command with captured stdout/stderr to prevent TUI corruption
fn run_captured_command(
    command: &str,
    mut output: Vec<String>,
    command_used: &str,
) -> InstallResult {
    use std::process::{Command, Stdio};

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

    let result = Command::new(&shell)
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .output();

    match result {
        Ok(out) => {
            // Capture stdout (limit lines to prevent memory issues)
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines().take(50) {
                output.push(format!("  [OUT] {}", line));
            }
            if stdout.lines().count() > 50 {
                output.push("  [OUT] ... (output truncated)".to_string());
            }

            // Capture stderr (limit lines)
            let stderr = String::from_utf8_lossy(&out.stderr);
            for line in stderr.lines().take(50) {
                // Filter out password prompts for security
                if !line.contains("password") && !line.contains("[sudo]") {
                    output.push(format!("  [ERR] {}", line));
                }
            }
            if stderr.lines().count() > 50 {
                output.push("  [ERR] ... (output truncated)".to_string());
            }

            if out.status.success() {
                output.push(format!("SUCCESS: {}", command_used));
                InstallResult {
                    success: true,
                    output,
                    command_used: command_used.to_string(),
                }
            } else {
                output.push(format!(
                    "FAILED: {} (exit code: {:?})",
                    command_used,
                    out.status.code()
                ));
                InstallResult {
                    success: false,
                    output,
                    command_used: command_used.to_string(),
                }
            }
        }
        Err(e) => {
            output.push(format!("ERROR: Failed to execute command: {}", e));
            InstallResult {
                success: false,
                output,
                command_used: command_used.to_string(),
            }
        }
    }
}

pub fn check_lsp_installed(lsp: &LspInfo) -> bool {
    which::which(lsp.binary_name).is_ok()
}

pub fn get_missing_lsps(installed: &[&str]) -> Vec<LspInfo> {
    get_available_lsps()
        .into_iter()
        .filter(|lsp| !installed.contains(&lsp.name))
        .collect()
}

pub fn get_languages_needing_lsp(
    detected_extensions: &[&str],
    installed_lsps: &[&str],
) -> Vec<LspInfo> {
    let mut needed: Vec<LspInfo> = Vec::new();
    let installed_set: std::collections::HashSet<&str> = installed_lsps.iter().copied().collect();

    for ext in detected_extensions {
        for lsp in get_lsp_for_extension(ext) {
            if !installed_set.contains(lsp.name)
                && !needed.iter().any(|l: &LspInfo| l.name == lsp.name) {
                    needed.push(lsp.clone());
                }
        }
    }

    needed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_available_lsps() {
        let lsps = get_available_lsps();
        assert!(!lsps.is_empty());
        assert!(lsps.iter().any(|l| l.name == "rust-analyzer"));
        assert!(lsps.iter().any(|l| l.name == "ruff"));
    }

    #[test]
    fn test_get_lsp_for_extension() {
        let rust_lsps = get_lsp_for_extension("rs");
        assert!(!rust_lsps.is_empty());
        assert!(rust_lsps.iter().any(|l| l.name == "rust-analyzer"));

        let py_lsps = get_lsp_for_extension("py");
        assert!(!py_lsps.is_empty());
        assert!(py_lsps.iter().any(|l| l.name == "ruff"));
    }

    #[test]
    fn test_system_package_command() {
        let lsps = get_available_lsps();
        let ruff = lsps.iter().find(|l| l.name == "ruff").unwrap();

        // Test Arch command
        let arch_cmd = get_install_command_for_distro(ruff, Distro::Arch);
        assert!(arch_cmd.contains("pacman"));
        assert!(arch_cmd.contains("ruff"));

        // Test Debian command
        let debian_cmd = get_install_command_for_distro(ruff, Distro::Debian);
        assert!(debian_cmd.contains("apt-get"));
        assert!(debian_cmd.contains("ruff"));
    }

    #[test]
    fn test_fallback_command() {
        let lsps = get_available_lsps();
        let taplo = lsps.iter().find(|l| l.name == "taplo").unwrap();

        // On Debian (no system package), should show cargo fallback
        let debian_cmd = get_install_command_for_distro(taplo, Distro::Debian);
        assert!(debian_cmd.contains("cargo install"));
    }
}
