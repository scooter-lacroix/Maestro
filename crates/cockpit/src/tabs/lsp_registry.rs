//! LSP Registry - Known Rust-based Language Servers
//!
//! This module contains the registry of LSP servers that can be installed
//! via cargo or other package managers. All listed LSPs are written in Rust
//! or have Rust-based implementations.

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

#[derive(Clone, Debug)]
pub enum LspInstallMethod {
    Cargo {
        crate_name: &'static str,
    },
    Rustup {
        component: &'static str,
    },
    Pip {
        package: &'static str,
    },
    Npm {
        package: &'static str,
    },
    Go {
        package: &'static str,
    },
    Custom {
        command: &'static str,
        notes: &'static str,
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
            install_method: LspInstallMethod::Cargo { crate_name: "ruff" },
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
            install_method: LspInstallMethod::Cargo {
                crate_name: "typescript-language-server",
            },
            binary_name: "typescript-language-server",
            lsp_command: "typescript-language-server --stdio",
            description: "Rust-based TypeScript/JavaScript language server",
            homepage: "https://github.com/quinnjr/typescript-language-server",
        },
        LspInfo {
            name: "taplo",
            display_name: "Taplo",
            language: "TOML",
            file_extensions: &["toml"],
            install_method: LspInstallMethod::Cargo {
                crate_name: "taplo-cli",
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
            install_method: LspInstallMethod::Go {
                package: "golang.org/x/tools/gopls@latest",
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

pub fn get_install_command(lsp: &LspInfo) -> String {
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
            if !installed_set.contains(lsp.name) {
                if !needed.iter().any(|l: &LspInfo| l.name == lsp.name) {
                    needed.push(lsp.clone());
                }
            }
        }
    }

    needed
}
