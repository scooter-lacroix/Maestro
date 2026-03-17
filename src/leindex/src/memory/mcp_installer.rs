//! Managed MCP installer for pool-local server lifecycles.
//!
//! This module installs MCP servers into a Maestro-managed root so they remain
//! available to the pool without relying on system-wide package installation.

#[cfg(feature = "rusqlite")]
use anyhow::{Context, Result};
#[cfg(feature = "rusqlite")]
use chrono::Utc;
#[cfg(feature = "rusqlite")]
use std::collections::HashMap;
#[cfg(feature = "rusqlite")]
use std::fs::{self, OpenOptions};
#[cfg(feature = "rusqlite")]
use std::io::Write;
#[cfg(feature = "rusqlite")]
use std::path::{Path, PathBuf};
#[cfg(feature = "rusqlite")]
use tokio::process::Command;

#[cfg(feature = "rusqlite")]
use crate::config::Config;

#[cfg(feature = "rusqlite")]
use super::models::{
    McpInstallKind, McpInstallState, McpManagedInstallManifest, McpManagedInstallRecipe,
    McpServer, McpStatus, McpTransport,
};
#[cfg(feature = "rusqlite")]
use super::service::MemoryService;

#[cfg(feature = "rusqlite")]
pub struct ManagedMcpInstaller {
    service: MemoryService,
    managed_root: PathBuf,
}

#[cfg(feature = "rusqlite")]
impl ManagedMcpInstaller {
    pub fn new(service: MemoryService, managed_root: Option<PathBuf>) -> Result<Self> {
        let root = match managed_root {
            Some(path) => path,
            None => default_managed_root()?,
        };
        fs::create_dir_all(&root)
            .with_context(|| format!("Failed to create managed MCP root {}", root.display()))?;
        Ok(Self {
            service,
            managed_root: root,
        })
    }

    pub fn template(&self, existing: Option<&McpServer>, suggested_name: Option<&str>) -> Result<String> {
        let manifest = if let Some(server) = existing {
            manifest_from_server(server)?
        } else {
            let name = suggested_name.unwrap_or("example-mcp").to_string();
            McpManagedInstallManifest {
                name,
                transport: McpTransport::Stdio,
                auto_start: true,
                env: serde_json::json!({
                    "API_KEY": { "value": "replace-me", "secret": true }
                }),
                recipe: McpManagedInstallRecipe {
                    kind: McpInstallKind::NpmPackage,
                    package: Some("@modelcontextprotocol/server-sequential-thinking".to_string()),
                    version: None,
                    binary: None,
                    python: None,
                    repository: None,
                    branch: None,
                    setup_commands: Vec::new(),
                    install_commands: Vec::new(),
                    build_commands: Vec::new(),
                    uninstall_commands: Vec::new(),
                    start_command: None,
                    start_args: vec![],
                    env: serde_json::json!({}),
                    cwd: None,
                    source_subdir: None,
                    post_install_command: None,
                    description: Some("Replace this template with your managed MCP recipe".to_string()),
                },
            }
        };

        toml::to_string_pretty(&manifest).context("Failed to render managed MCP manifest")
    }

    pub async fn install_from_manifest_path(&self, path: &Path) -> Result<McpServer> {
        let manifest_toml = fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest {}", path.display()))?;
        self.install_from_manifest_str(&manifest_toml).await
    }

    pub async fn install_from_manifest_str(&self, manifest_toml: &str) -> Result<McpServer> {
        let manifest: McpManagedInstallManifest =
            toml::from_str(manifest_toml).context("Invalid managed MCP manifest TOML")?;
        manifest.recipe.validate(&manifest.name)?;

        let server_root = self.managed_root_for(&manifest.name);
        if server_root.exists() {
            let _ = self.uninstall(&manifest.name).await;
        }

        fs::create_dir_all(server_root.join("runtime")).with_context(|| {
            format!(
                "Failed to create managed runtime directory {}",
                server_root.display()
            )
        })?;
        fs::create_dir_all(server_root.join("workspace")).with_context(|| {
            format!(
                "Failed to create managed workspace directory {}",
                server_root.display()
            )
        })?;

        let log_path = server_root.join("install.log");
        self.write_install_state(
            &manifest,
            server_root.to_string_lossy().as_ref(),
            log_path.to_string_lossy().as_ref(),
            manifest_toml,
            McpInstallState::Installing,
            Some("Installing managed MCP server".to_string()),
        )?;

        let install_result = self
            .install_manifest(&manifest, &server_root, &log_path, manifest_toml)
            .await;

        match install_result {
            Ok(server) => Ok(server),
            Err(error) => {
                self.write_install_state(
                    &manifest,
                    server_root.to_string_lossy().as_ref(),
                    log_path.to_string_lossy().as_ref(),
                    manifest_toml,
                    McpInstallState::Failed,
                    Some(error.to_string()),
                )?;
                Err(error)
            }
        }
    }

    pub async fn reinstall(&self, name: &str) -> Result<McpServer> {
        let server = self
            .service
            .list_mcp_servers()?
            .into_iter()
            .find(|server| server.name == name)
            .with_context(|| format!("Managed MCP server '{}' not found", name))?;
        if !server.managed {
            anyhow::bail!("MCP server '{}' is not managed by Maestro", name);
        }
        let manifest = self.template(Some(&server), Some(name))?;
        self.install_from_manifest_str(&manifest).await
    }

    pub async fn uninstall(&self, name: &str) -> Result<()> {
        let maybe_server = self
            .service
            .list_mcp_servers()?
            .into_iter()
            .find(|server| server.name == name);

        let Some(server) = maybe_server else {
            return Ok(());
        };

        if !server.managed {
            self.service.delete_mcp_server(name)?;
            return Ok(());
        }

        let recipe = server
            .install_recipe
            .clone()
            .and_then(|recipe| serde_json::from_value::<McpManagedInstallRecipe>(recipe).ok());

        let root = server
            .install_root
            .clone()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.managed_root_for(name));
        let log_path = server
            .install_log_path
            .clone()
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("install.log"));

        if let Some(recipe) = recipe {
            let _ = self
                .run_command_group(
                    &recipe.uninstall_commands,
                    &root.join("workspace"),
                    &json_env_to_hashmap(&server.env),
                    &log_path,
                )
                .await;
        }

        if root.exists() {
            fs::remove_dir_all(&root)
                .with_context(|| format!("Failed to remove managed MCP root {}", root.display()))?;
        }

        self.service.purge_mcp_server_record(name)?;
        Ok(())
    }

    pub fn managed_root_for(&self, name: &str) -> PathBuf {
        self.managed_root.join(sanitize_name(name))
    }

    async fn install_manifest(
        &self,
        manifest: &McpManagedInstallManifest,
        server_root: &Path,
        log_path: &Path,
        manifest_toml: &str,
    ) -> Result<McpServer> {
        let workspace = server_root.join("workspace");
        let runtime_dir = server_root.join("runtime");
        let env_map = json_env_to_hashmap(&manifest.env);

        let recipe = &manifest.recipe;
        let run_spec = match recipe.kind {
            McpInstallKind::NpmPackage => {
                self.run_command_group(&recipe.setup_commands, &workspace, &env_map, log_path)
                    .await?;
                initialize_npm_workspace(&workspace)?;

                let package = recipe
                    .package
                    .clone()
                    .with_context(|| "NPM package installs require a package name")?;
                let mut args = vec!["install".to_string(), package.clone()];
                args.extend(recipe.install_commands.clone());
                run_process("npm", &args, &workspace, &env_map, log_path).await?;
                if let Some(command) = recipe.post_install_command.as_ref() {
                    run_shell_group(
                        std::slice::from_ref(command),
                        &workspace,
                        &env_map,
                        log_path,
                    )
                    .await?;
                }

                let bin =
                    resolve_npm_bin(&workspace, &package, recipe.binary.as_deref())?;
                ManagedExec::Direct {
                    command: workspace.join("node_modules").join(".bin").join(bin),
                    args: recipe.start_args.clone(),
                    cwd: workspace.clone(),
                }
            }
            McpInstallKind::UvxPackage => {
                self.run_command_group(&recipe.setup_commands, &workspace, &env_map, log_path)
                    .await?;
                let venv = workspace.join("venv");
                let mut venv_args = vec!["venv".to_string(), venv.to_string_lossy().to_string()];
                if let Some(python) = recipe.python.as_ref() {
                    venv_args.push("--python".to_string());
                    venv_args.push(python.clone());
                }
                run_process("uv", &venv_args, &workspace, &env_map, log_path).await?;

                let pip = venv_bin(&venv, "pip");
                let package = recipe
                    .package
                    .clone()
                    .with_context(|| "UVX installs require a package name")?;
                let mut pip_args = vec!["install".to_string(), package.clone()];
                pip_args.extend(recipe.install_commands.clone());
                run_process(pip.to_string_lossy().as_ref(), &pip_args, &workspace, &env_map, log_path)
                    .await?;
                if let Some(command) = recipe.post_install_command.as_ref() {
                    run_shell_group(
                        std::slice::from_ref(command),
                        &workspace,
                        &env_map,
                        log_path,
                    )
                    .await?;
                }

                let bin = recipe
                    .binary
                    .clone()
                    .unwrap_or_else(|| default_python_bin(package.as_str()));
                ManagedExec::Direct {
                    command: venv_bin(&venv, &bin),
                    args: recipe.start_args.clone(),
                    cwd: workspace.clone(),
                }
            }
            McpInstallKind::PipxPackage => {
                self.run_command_group(&recipe.setup_commands, &workspace, &env_map, log_path)
                    .await?;
                let venv = workspace.join("venv");
                let python_cmd = recipe
                    .python
                    .clone()
                    .unwrap_or_else(|| "python3".to_string());
                run_process(
                    &python_cmd,
                    &["-m".to_string(), "venv".to_string(), venv.to_string_lossy().to_string()],
                    &workspace,
                    &env_map,
                    log_path,
                )
                .await?;

                let pip = venv_bin(&venv, "pip");
                let package = recipe
                    .package
                    .clone()
                    .with_context(|| "PIPX installs require a package name")?;
                let mut pip_args = vec!["install".to_string(), package.clone()];
                pip_args.extend(recipe.install_commands.clone());
                run_process(pip.to_string_lossy().as_ref(), &pip_args, &workspace, &env_map, log_path)
                    .await?;
                if let Some(command) = recipe.post_install_command.as_ref() {
                    run_shell_group(
                        std::slice::from_ref(command),
                        &workspace,
                        &env_map,
                        log_path,
                    )
                    .await?;
                }

                let bin = recipe
                    .binary
                    .clone()
                    .unwrap_or_else(|| default_python_bin(package.as_str()));
                ManagedExec::Direct {
                    command: venv_bin(&venv, &bin),
                    args: recipe.start_args.clone(),
                    cwd: workspace.clone(),
                }
            }
            McpInstallKind::GitRepository => {
                let repository = recipe
                    .repository
                    .clone()
                    .with_context(|| "Git repository installs require a repository URL")?;
                let source_dir = workspace.join("source");
                run_process(
                    "git",
                    &["clone".to_string(), repository, source_dir.to_string_lossy().to_string()],
                    &workspace,
                    &env_map,
                    log_path,
                )
                .await?;
                if let Some(branch) = recipe.branch.as_ref() {
                    run_process(
                        "git",
                        &["checkout".to_string(), branch.clone()],
                        &source_dir,
                        &env_map,
                        log_path,
                    )
                    .await?;
                }
                let working_dir = recipe
                    .source_subdir
                    .as_ref()
                    .map(|subdir| source_dir.join(subdir))
                    .unwrap_or_else(|| source_dir.clone());
                run_shell_group(&recipe.setup_commands, &working_dir, &env_map, log_path).await?;
                run_shell_group(&recipe.install_commands, &working_dir, &env_map, log_path).await?;
                run_shell_group(&recipe.build_commands, &working_dir, &env_map, log_path).await?;
                if let Some(command) = recipe.post_install_command.as_ref() {
                    run_shell_group(
                        std::slice::from_ref(command),
                        &working_dir,
                        &env_map,
                        log_path,
                    )
                    .await?;
                }
                ManagedExec::Shell {
                    script: recipe
                        .start_command
                        .clone()
                        .with_context(|| "Git repository installs require a start command")?,
                    args: recipe.start_args.clone(),
                    cwd: working_dir,
                }
            }
            McpInstallKind::Custom => {
                let working_dir = recipe
                    .cwd
                    .as_ref()
                    .map(|value| workspace.join(value))
                    .unwrap_or_else(|| workspace.clone());
                run_shell_group(&recipe.setup_commands, &working_dir, &env_map, log_path).await?;
                run_shell_group(&recipe.install_commands, &working_dir, &env_map, log_path).await?;
                run_shell_group(&recipe.build_commands, &working_dir, &env_map, log_path).await?;
                if let Some(command) = recipe.post_install_command.as_ref() {
                    run_shell_group(
                        std::slice::from_ref(command),
                        &working_dir,
                        &env_map,
                        log_path,
                    )
                    .await?;
                }
                ManagedExec::Shell {
                    script: recipe
                        .start_command
                        .clone()
                        .with_context(|| "Custom installs require a start command")?,
                    args: recipe.start_args.clone(),
                    cwd: working_dir,
                }
            }
            McpInstallKind::Unmanaged => anyhow::bail!("Managed install recipe cannot be unmanaged"),
        };

        let wrapper_path = runtime_dir.join("launch.sh");
        write_wrapper_script(&wrapper_path, &run_spec, &env_map)?;
        fs::write(
            server_root.join("manifest.toml"),
            manifest_toml.as_bytes(),
        )
        .with_context(|| format!("Failed to write manifest for {}", manifest.name))?;

        let recipe_json = serde_json::to_value(&manifest.recipe)?;
        let now = Utc::now();
        let server = McpServer {
            id: 0,
            name: manifest.name.clone(),
            transport: manifest.transport,
            command: wrapper_path.to_string_lossy().to_string(),
            args: Vec::new(),
            env: manifest.env.clone(),
            cwd: None,
            url: None,
            headers: None,
            status: McpStatus::Stopped,
            socket_path: None,
            client_count: 0,
            last_started_at: None,
            managed: true,
            install_type: manifest.recipe.kind(),
            install_state: McpInstallState::Installed,
            install_root: Some(server_root.to_string_lossy().to_string()),
            install_recipe: Some(recipe_json),
            install_message: Some(format!("Installed via {}", manifest.recipe.kind())),
            install_log_path: Some(log_path.to_string_lossy().to_string()),
            last_install_at: Some(now),
        };
        self.service.unblock_mcp_server(&manifest.name)?;
        self.service.update_mcp_server(server.clone())?;
        Ok(server)
    }

    fn write_install_state(
        &self,
        manifest: &McpManagedInstallManifest,
        install_root: &str,
        log_path: &str,
        manifest_toml: &str,
        state: McpInstallState,
        message: Option<String>,
    ) -> Result<()> {
        let recipe_json = serde_json::to_value(&manifest.recipe)?;
        let server = McpServer {
            id: 0,
            name: manifest.name.clone(),
            transport: manifest.transport,
            command: self
            .managed_root_for(&manifest.name)
                .join("runtime")
                .join("launch.sh")
                .to_string_lossy()
                .to_string(),
            args: Vec::new(),
            env: manifest.env.clone(),
            cwd: None,
            url: None,
            headers: None,
            status: McpStatus::Stopped,
            socket_path: None,
            client_count: 0,
            last_started_at: None,
            managed: true,
            install_type: manifest.recipe.kind(),
            install_state: state,
            install_root: Some(install_root.to_string()),
            install_recipe: Some(recipe_json),
            install_message: message,
            install_log_path: Some(log_path.to_string()),
            last_install_at: Some(Utc::now()),
        };
        fs::write(
            self.managed_root_for(&manifest.name).join("manifest.toml"),
            manifest_toml.as_bytes(),
        )
        .ok();
        self.service.unblock_mcp_server(&manifest.name)?;
        self.service.update_mcp_server(server)?;
        Ok(())
    }

    async fn run_command_group(
        &self,
        commands: &[String],
        cwd: &Path,
        env: &HashMap<String, String>,
        log_path: &Path,
    ) -> Result<()> {
        run_shell_group(commands, cwd, env, log_path).await
    }
}

#[cfg(feature = "rusqlite")]
enum ManagedExec {
    Direct {
        command: PathBuf,
        args: Vec<String>,
        cwd: PathBuf,
    },
    Shell {
        script: String,
        args: Vec<String>,
        cwd: PathBuf,
    },
}

#[cfg(feature = "rusqlite")]
fn default_managed_root() -> Result<PathBuf> {
    let config = Config::load();
    let root = expand_user_path(&config.install_path)?.join("mcp").join("managed");
    Ok(root)
}

#[cfg(feature = "rusqlite")]
fn manifest_from_server(server: &McpServer) -> Result<McpManagedInstallManifest> {
    if !server.managed {
        return Ok(manifest_from_unmanaged_server(server));
    }

    let recipe = server
        .install_recipe
        .clone()
        .with_context(|| format!("Managed MCP '{}' has no stored recipe", server.name))
        .and_then(|value| serde_json::from_value(value).context("Invalid stored managed MCP recipe"))?;

    Ok(McpManagedInstallManifest {
        name: server.name.clone(),
        transport: server.transport,
        auto_start: true,
        env: server.env.clone(),
        recipe,
    })
}

#[cfg(feature = "rusqlite")]
fn manifest_from_unmanaged_server(server: &McpServer) -> McpManagedInstallManifest {
    let recipe = infer_recipe_from_server(server);
    McpManagedInstallManifest {
        name: server.name.clone(),
        transport: server.transport,
        auto_start: true,
        env: server.env.clone(),
        recipe,
    }
}

#[cfg(feature = "rusqlite")]
fn infer_recipe_from_server(server: &McpServer) -> McpManagedInstallRecipe {
    if let Some(recipe) = infer_npx_recipe(server) {
        return recipe;
    }
    if let Some(recipe) = infer_uvx_recipe(server) {
        return recipe;
    }

    McpManagedInstallRecipe {
        kind: McpInstallKind::Custom,
        package: None,
        version: None,
        binary: None,
        python: None,
        repository: None,
        branch: None,
        setup_commands: Vec::new(),
        install_commands: Vec::new(),
        build_commands: Vec::new(),
        uninstall_commands: Vec::new(),
        start_command: Some(server.command.clone()),
        start_args: server.args.clone(),
        env: server.env.clone(),
        cwd: server.cwd.clone(),
        source_subdir: None,
        post_install_command: None,
        description: Some(
            "Fill in install/setup/build commands so Maestro can manage this server locally."
                .to_string(),
        ),
    }
}

#[cfg(feature = "rusqlite")]
fn infer_npx_recipe(server: &McpServer) -> Option<McpManagedInstallRecipe> {
    if server.transport != McpTransport::Stdio || server.command != "npx" {
        return None;
    }

    let mut package = None;
    let mut start_args = Vec::new();
    let mut after_package = false;
    for arg in &server.args {
        if after_package {
            start_args.push(arg.clone());
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        package = Some(arg.clone());
        after_package = true;
    }

    let package = package?;
    Some(McpManagedInstallRecipe {
        kind: McpInstallKind::NpmPackage,
        package: Some(package),
        version: None,
        binary: None,
        python: None,
        repository: None,
        branch: None,
        setup_commands: Vec::new(),
        install_commands: Vec::new(),
        build_commands: Vec::new(),
        uninstall_commands: Vec::new(),
        start_command: None,
        start_args,
        env: server.env.clone(),
        cwd: server.cwd.clone(),
        source_subdir: None,
        post_install_command: None,
        description: Some(
            "Generated from an existing npx MCP definition; edit if the package needs setup or a custom bin."
                .to_string(),
        ),
    })
}

#[cfg(feature = "rusqlite")]
fn infer_uvx_recipe(server: &McpServer) -> Option<McpManagedInstallRecipe> {
    if server.transport != McpTransport::Stdio || server.command != "uvx" {
        return None;
    }

    let mut package = None;
    let mut start_args = Vec::new();
    let mut after_package = false;
    for arg in &server.args {
        if after_package {
            start_args.push(arg.clone());
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        package = Some(arg.clone());
        after_package = true;
    }

    let package = package?;
    Some(McpManagedInstallRecipe {
        kind: McpInstallKind::UvxPackage,
        package: Some(package),
        version: None,
        binary: None,
        python: None,
        repository: None,
        branch: None,
        setup_commands: Vec::new(),
        install_commands: Vec::new(),
        build_commands: Vec::new(),
        uninstall_commands: Vec::new(),
        start_command: None,
        start_args,
        env: server.env.clone(),
        cwd: server.cwd.clone(),
        source_subdir: None,
        post_install_command: None,
        description: Some(
            "Generated from an existing uvx MCP definition; edit if the package needs setup or a custom binary."
                .to_string(),
        ),
    })
}

#[cfg(feature = "rusqlite")]
fn expand_user_path(path: &str) -> Result<PathBuf> {
    if let Some(stripped) = path.strip_prefix("~/") {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        return Ok(home.join(stripped));
    }
    Ok(PathBuf::from(path))
}

#[cfg(feature = "rusqlite")]
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(feature = "rusqlite")]
fn initialize_npm_workspace(workspace: &Path) -> Result<()> {
    let package_json = workspace.join("package.json");
    if !package_json.exists() {
        fs::write(
            &package_json,
            br#"{"name":"maestro-managed-mcp","private":true}"#,
        )
        .with_context(|| format!("Failed to initialize npm workspace {}", workspace.display()))?;
    }
    Ok(())
}

#[cfg(feature = "rusqlite")]
fn resolve_npm_bin(workspace: &Path, package_spec: &str, explicit_bin: Option<&str>) -> Result<String> {
    if let Some(bin) = explicit_bin {
        return Ok(bin.to_string());
    }

    let package_name = normalize_npm_package_name(package_spec);
    let package_json = workspace.join("node_modules").join(package_name).join("package.json");
    let package_text = fs::read_to_string(&package_json)
        .with_context(|| format!("Failed to read {}", package_json.display()))?;
    let package_value: serde_json::Value =
        serde_json::from_str(&package_text).context("Invalid package.json for npm MCP package")?;

    if let Some(bin) = package_value.get("bin") {
        if let Some(single) = bin.as_str() {
            let fallback = package_name
                .rsplit('/')
                .next()
                .unwrap_or(package_name)
                .to_string();
            return Ok(
                Path::new(single)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(&fallback)
                    .to_string(),
            );
        }
        if let Some(map) = bin.as_object() {
            if let Some((name, _)) = map.iter().next() {
                return Ok(name.clone());
            }
        }
    }

    Ok(package_name
        .rsplit('/')
        .next()
        .unwrap_or(package_name)
        .to_string())
}

#[cfg(feature = "rusqlite")]
fn normalize_npm_package_name(package_spec: &str) -> &str {
    if let Some(stripped) = package_spec.strip_prefix('@') {
        if let Some(version_offset) = stripped.rfind('@') {
            return &package_spec[..(version_offset + 1)];
        }
        package_spec
    } else {
        package_spec.split_once('@').map(|(name, _)| name).unwrap_or(package_spec)
    }
}

#[cfg(feature = "rusqlite")]
fn default_python_bin(package_spec: &str) -> String {
    package_spec
        .split(['@', '['])
        .next()
        .unwrap_or(package_spec)
        .rsplit('/')
        .next()
        .unwrap_or(package_spec)
        .replace('_', "-")
}

#[cfg(feature = "rusqlite")]
fn venv_bin(venv: &Path, name: &str) -> PathBuf {
    if cfg!(target_os = "windows") {
        venv.join("Scripts").join(format!("{}.exe", name))
    } else {
        venv.join("bin").join(name)
    }
}

#[cfg(feature = "rusqlite")]
fn write_wrapper_script(
    path: &Path,
    exec: &ManagedExec,
    env: &HashMap<String, String>,
) -> Result<()> {
    let mut script = String::from("#!/usr/bin/env bash\nset -euo pipefail\n");
    for (key, value) in env {
        script.push_str(&format!("export {}={}\n", key, shell_quote(value)));
    }
    match exec {
        ManagedExec::Direct { command, args, cwd } => {
            script.push_str(&format!("cd {}\n", shell_quote(&cwd.to_string_lossy())));
            script.push_str("exec ");
            script.push_str(&shell_quote(&command.to_string_lossy()));
            for arg in args {
                script.push(' ');
                script.push_str(&shell_quote(arg));
            }
            script.push('\n');
        }
        ManagedExec::Shell { script: shell, args, cwd } => {
            script.push_str(&format!("cd {}\n", shell_quote(&cwd.to_string_lossy())));
            let mut rendered = shell.clone();
            if !args.is_empty() {
                rendered.push(' ');
                rendered.push_str(
                    &args.iter()
                        .map(|arg| shell_quote(arg))
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
            script.push_str(&format!("exec bash -lc {}\n", shell_quote(&rendered)));
        }
    }

    fs::write(path, script.as_bytes())
        .with_context(|| format!("Failed to write wrapper script {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    Ok(())
}

#[cfg(feature = "rusqlite")]
fn json_env_to_hashmap(value: &serde_json::Value) -> HashMap<String, String> {
    let mut env = HashMap::new();
    if let Some(map) = value.as_object() {
        for (key, entry) in map {
            if let Some(object) = entry.as_object() {
                if let Some(stored) = object.get("value").and_then(|value| value.as_str()) {
                    env.insert(key.clone(), stored.to_string());
                    continue;
                }
            }
            if let Some(text) = entry.as_str() {
                env.insert(key.clone(), text.to_string());
            } else {
                env.insert(key.clone(), entry.to_string());
            }
        }
    }
    env
}

#[cfg(feature = "rusqlite")]
async fn run_shell_group(
    commands: &[String],
    cwd: &Path,
    env: &HashMap<String, String>,
    log_path: &Path,
) -> Result<()> {
    for command in commands {
        run_process("bash", &["-lc".to_string(), command.clone()], cwd, env, log_path).await?;
    }
    Ok(())
}

#[cfg(feature = "rusqlite")]
async fn run_process(
    program: &str,
    args: &[String],
    cwd: &Path,
    env: &HashMap<String, String>,
    log_path: &Path,
) -> Result<()> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .envs(env)
        .output()
        .await
        .with_context(|| format!("Failed to run '{}' in {}", program, cwd.display()))?;

    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("Failed to open install log {}", log_path.display()))?;
    writeln!(log, "$ {} {}", program, args.join(" ")).ok();
    log.write_all(&output.stdout).ok();
    log.write_all(&output.stderr).ok();
    writeln!(log).ok();

    if !output.status.success() {
        anyhow::bail!(
            "Command '{}' failed with status {}",
            program,
            output.status
        );
    }

    Ok(())
}

#[cfg(feature = "rusqlite")]
fn shell_quote(value: &str) -> String {
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{}'", escaped)
}

#[cfg(all(test, feature = "rusqlite"))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn custom_recipe_install_and_uninstall_manages_root() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let service = MemoryService::new(Some(db_path)).unwrap();
        service.initialize().unwrap();
        let installer =
            ManagedMcpInstaller::new(service.clone(), Some(temp.path().join("managed"))).unwrap();

        let manifest = r#"
name = "echo-cat"
auto_start = false

[recipe]
kind = "custom"
install_commands = ["printf '#!/usr/bin/env bash\ncat\n' > server.sh", "chmod +x server.sh"]
start_command = "./server.sh"
"#;

        let server = installer.install_from_manifest_str(manifest).await.unwrap();
        assert!(server.managed);
        assert_eq!(server.install_state, McpInstallState::Installed);
        assert!(Path::new(server.install_root.as_deref().unwrap()).exists());

        installer.uninstall("echo-cat").await.unwrap();
        let remaining = service
            .list_mcp_servers()
            .unwrap()
            .into_iter()
            .find(|entry| entry.name == "echo-cat");
        assert!(remaining.is_none());
    }

    #[test]
    fn template_generation_supports_unmanaged_npx_servers() {
        let server = McpServer {
            id: 0,
            name: "sequential-thinking".to_string(),
            transport: McpTransport::Stdio,
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-sequential-thinking".to_string(),
            ],
            env: serde_json::json!({}),
            cwd: None,
            url: None,
            headers: None,
            status: McpStatus::Stopped,
            socket_path: None,
            client_count: 0,
            last_started_at: None,
            managed: false,
            install_type: McpInstallKind::Unmanaged,
            install_state: McpInstallState::Unmanaged,
            install_root: None,
            install_recipe: None,
            install_message: None,
            install_log_path: None,
            last_install_at: None,
        };

        let manifest = manifest_from_server(&server).unwrap();
        assert_eq!(manifest.recipe.kind, McpInstallKind::NpmPackage);
        assert_eq!(
            manifest.recipe.package.as_deref(),
            Some("@modelcontextprotocol/server-sequential-thinking")
        );
    }
}
