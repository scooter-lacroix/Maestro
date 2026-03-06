//! User-defined skill management for MaestroClaw.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tools: Vec<SkillTool>,
    #[serde(default)]
    pub prompts: Vec<String>,
    #[serde(skip)]
    pub location: Option<PathBuf>,
}

fn default_version() -> String {
    "0.1.0".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    pub name: String,
    pub description: String,
    pub kind: String,
    pub command: String,
    #[serde(default)]
    pub args: HashMap<String, String>,
}

pub fn load_skills(workspace_dir: &Path) -> Vec<Skill> {
    let dir = workspace_dir.join("skills");
    if !dir.exists() {
        return Vec::new();
    }

    let mut skills = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let manifest = path.join("SKILL.toml");
            if !manifest.exists() {
                continue;
            }

            if let Ok(mut skill) = load_manifest(&manifest) {
                skill.location = Some(path);
                skills.push(skill);
            }
        }
    }

    skills
}

fn load_manifest(path: &Path) -> Result<Skill> {
    #[derive(Deserialize)]
    struct Manifest {
        skill: ManifestSkill,
        #[serde(default)]
        tools: Vec<SkillTool>,
        #[serde(default)]
        prompts: Vec<String>,
    }

    #[derive(Deserialize)]
    struct ManifestSkill {
        name: String,
        description: String,
        #[serde(default = "default_version")]
        version: String,
        #[serde(default)]
        author: Option<String>,
        #[serde(default)]
        tags: Vec<String>,
    }

    let content = std::fs::read_to_string(path)?;
    let manifest: Manifest = toml::from_str(&content)?;

    Ok(Skill {
        name: manifest.skill.name,
        description: manifest.skill.description,
        version: manifest.skill.version,
        author: manifest.skill.author,
        tags: manifest.skill.tags,
        tools: manifest.tools,
        prompts: manifest.prompts,
        location: None,
    })
}

pub fn install_skill(workspace_dir: &Path, source: &Path) -> Result<()> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid source path"))?;
    let destination = workspace_dir.join("skills").join(name);

    if destination.exists() {
        anyhow::bail!("Skill '{name}' already installed");
    }

    copy_dir(source, &destination)?;
    println!("Installed skill: {name}");
    Ok(())
}

pub fn remove_skill(workspace_dir: &Path, name: &str) -> Result<()> {
    let path = workspace_dir.join("skills").join(name);
    if !path.exists() {
        anyhow::bail!("Skill '{name}' not found");
    }

    std::fs::remove_dir_all(&path)?;
    println!("Removed skill: {name}");
    Ok(())
}

fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)?;

    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = destination.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

pub fn handle_command(command: &str, args: &[&str], workspace_dir: &Path) -> Result<()> {
    match command {
        "list" => {
            let skills = load_skills(workspace_dir);
            if skills.is_empty() {
                println!("No skills installed.");
                return Ok(());
            }

            println!("Installed skills ({}):", skills.len());
            for skill in &skills {
                println!(
                    "  {} v{} - {}",
                    skill.name, skill.version, skill.description
                );
            }
            Ok(())
        }
        "install" if !args.is_empty() => install_skill(workspace_dir, Path::new(args[0])),
        "remove" if !args.is_empty() => remove_skill(workspace_dir, args[0]),
        _ => {
            println!("Usage: maestro claw skills [list|install|remove]");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_skills_returns_empty_when_none_installed() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(load_skills(tmp.path()).is_empty());
    }

    #[test]
    fn load_skills_reads_manifest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().join("skills").join("test-skill");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.toml"),
            r#"
[skill]
name = "test-skill"
description = "A test"
version = "1.0.0"

[[tools]]
name = "greet"
description = "Say hello"
kind = "shell"
command = "echo hello"
"#,
        )
        .unwrap();

        let skills = load_skills(tmp.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");
        assert_eq!(skills[0].tools.len(), 1);
    }
}
