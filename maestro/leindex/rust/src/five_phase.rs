//! 5-Phase Analysis System
//!
//! Token-efficient multi-file analysis helpers for Maestro/Leindex workflows.

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::token_format::{FormatMode, TokenFormatter};
use crate::{MultiLangASTAnalyzer, MultiLangCFGAnalyzer, MultiLangCallGraphAnalyzer, ProgrammingLanguage};

/// Directories to skip during analysis scans.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "__pycache__",
    "venv",
    "env",
    ".git",
    "dist",
    "build",
    "target",
    ".cargo",
    ".rustup",
    ".cache",
    ".npm",
    ".yarn",
    "vendor",
    ".venv",
    "site-packages",
];

/// Options shared by all phases.
#[derive(Debug, Clone)]
pub struct PhaseOptions {
    pub root: PathBuf,
    pub mode: FormatMode,
    pub max_files: usize,
    pub max_focus_files: usize,
    pub top_n: usize,
    pub max_output_chars: usize,
}

impl PhaseOptions {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            mode: FormatMode::Ultra,
            max_files: 25,
            max_focus_files: 3,
            top_n: 15,
            max_output_chars: 12_000,
        }
    }
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|name| SKIP_DIRS.iter().any(|skip| skip.eq_ignore_ascii_case(name)))
}

fn is_supported_source_file(path: &Path) -> bool {
    path.is_file() && ProgrammingLanguage::from_path(&path.to_string_lossy()).is_some()
}

fn file_priority(path: &Path) -> i32 {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut score = match name.as_str() {
        "main.rs" | "lib.rs" | "mod.rs" => 100,
        "index.ts" | "index.js" | "app.ts" | "app.js" => 90,
        _ => 0,
    };

    let path_str = path.to_string_lossy().to_ascii_lowercase();
    if path_str.contains("/src/") {
        score += 20;
    }
    if path_str.contains("/test") || path_str.contains("/tests") {
        score -= 10;
    }
    score
}

fn display_path(root: &Path, file: &Path) -> String {
    if root.is_file() {
        return file.to_string_lossy().to_string();
    }
    file.strip_prefix(root)
        .map(|p| format!("./{}", p.to_string_lossy()))
        .unwrap_or_else(|_| file.to_string_lossy().to_string())
}

fn collect_source_files(root: &Path, max_files: usize) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }

    let mut files: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !should_skip_dir(e.path()))
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if is_supported_source_file(path) {
            if let Ok(meta) = std::fs::metadata(path) {
                if meta.len() > crate::MAX_FILE_SIZE as u64 {
                    continue;
                }
            }
            files.push(path.to_path_buf());
        }
    }

    files.sort_by(|a, b| {
        let pa = file_priority(a);
        let pb = file_priority(b);
        pb.cmp(&pa)
            .then_with(|| a.as_os_str().len().cmp(&b.as_os_str().len()))
            .then_with(|| a.to_string_lossy().cmp(&b.to_string_lossy()))
    });

    files.truncate(max_files);
    Ok(files)
}

fn truncate_output(s: String, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s;
    }
    TokenFormatter::new().truncate(&s, max_chars)
}

fn format_block(mode: FormatMode, formatter: &TokenFormatter, s: String, max_chars: usize) -> String {
    match mode {
        FormatMode::Verbose => truncate_output(s, max_chars),
        FormatMode::Balanced => truncate_output(s, max_chars.min(6000)),
        FormatMode::Ultra => formatter.truncate(&s, max_chars.min(2500)),
    }
}

pub fn phase1_structural_scan(opts: &PhaseOptions) -> Result<String> {
    let formatter = TokenFormatter::new();
    let root = opts.root.clone();
    let files = collect_source_files(&root, opts.max_files)?;

    let mut lang_counts: HashMap<String, usize> = HashMap::new();
    let mut blocks: Vec<String> = Vec::new();
    let mut read_errors: usize = 0;

    let mut analyzer = MultiLangASTAnalyzer::new();
    for file in &files {
        let lang = match ProgrammingLanguage::from_path(&file.to_string_lossy()) {
            Some(l) => l,
            None => continue,
        };
        *lang_counts.entry(lang.display_name().to_string()).or_insert(0) += 1;

        let src = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => {
                read_errors += 1;
                continue;
            }
        };
        let shown_path = display_path(&root, file);
        let analysis = analyzer.analyze_with_language(&src, &shown_path, lang);

        let block = match opts.mode {
            FormatMode::Ultra => analyzer.to_ultra_condensed(&analysis),
            _ => analyzer.to_llm_string(&analysis),
        };
        blocks.push(format_block(opts.mode, &formatter, block, 4000));
    }

    let mut out = String::new();
    out.push_str("# /phase1 Structural Scan\n");
    out.push_str(&format!("root: {}\n", root.display()));
    out.push_str(&format!("files_analyzed: {}\n", blocks.len()));
    if read_errors > 0 {
        out.push_str(&format!("read_errors: {}\n", read_errors));
    }
    if !lang_counts.is_empty() {
        let mut langs: Vec<(String, usize)> = lang_counts.into_iter().collect();
        langs.sort_by(|a, b| b.1.cmp(&a.1));
        out.push_str("languages:");
        for (lang, count) in langs {
            out.push_str(&format!(" {}={}", lang, count));
        }
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&blocks.join("\n\n"));

    Ok(truncate_output(out, opts.max_output_chars))
}

pub fn phase2_dependency_map(opts: &PhaseOptions) -> Result<String> {
    let formatter = TokenFormatter::new();
    let root = opts.root.clone();
    let files = collect_source_files(&root, opts.max_files)?;

    let mut analyzer = MultiLangASTAnalyzer::new();
    let mut dep_freq: HashMap<String, usize> = HashMap::new();
    let mut lines: Vec<String> = Vec::new();
    let mut read_errors: usize = 0;

    for file in &files {
        let lang = match ProgrammingLanguage::from_path(&file.to_string_lossy()) {
            Some(l) => l,
            None => continue,
        };
        let src = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => {
                read_errors += 1;
                continue;
            }
        };
        let shown_path = display_path(&root, file);
        let analysis = analyzer.analyze_with_language(&src, &shown_path, lang);

        if analysis.imports.is_empty() {
            continue;
        }

        let mut deps: Vec<String> = Vec::new();
        for imp in analysis.imports.iter().take(20) {
            let module = formatter.truncate(&imp.module, 50);
            *dep_freq.entry(module.clone()).or_insert(0) += 1;
            deps.push(module);
        }
        deps.sort();
        deps.dedup();

        lines.push(format!("{} -> {}", shown_path, deps.join(" ")));
    }

    let mut out = String::new();
    out.push_str("# /phase2 Dependency Map\n");
    out.push_str(&format!("root: {}\n", root.display()));
    if read_errors > 0 {
        out.push_str(&format!("read_errors: {}\n", read_errors));
    }

    if !dep_freq.is_empty() {
        let mut top: Vec<(String, usize)> = dep_freq.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        out.push_str("top_deps:");
        for (module, count) in top.into_iter().take(12) {
            out.push_str(&format!(" {}({})", module, count));
        }
        out.push('\n');
    }

    out.push('\n');
    if lines.is_empty() {
        out.push_str("(no imports detected in scanned files)\n");
        return Ok(out);
    }

    for line in lines {
        out.push_str(&formatter.truncate(&line, 800));
        out.push('\n');
    }

    Ok(truncate_output(out, opts.max_output_chars))
}

pub fn phase3_logic_flow(opts: &PhaseOptions) -> Result<String> {
    let formatter = TokenFormatter::new();
    let root = opts.root.clone();
    let files = collect_source_files(&root, opts.max_focus_files)?;

    let mut cg = MultiLangCallGraphAnalyzer::new();
    let mut cfg = MultiLangCFGAnalyzer::new();

    let mut blocks: Vec<String> = Vec::new();
    let mut read_errors: usize = 0;
    for file in &files {
        let lang = match ProgrammingLanguage::from_path(&file.to_string_lossy()) {
            Some(l) => l,
            None => continue,
        };
        let src = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => {
                read_errors += 1;
                continue;
            }
        };
        let shown_path = display_path(&root, file);

        let graph = cg.build_graph_with_language(&src, &shown_path, lang);
        let cfg_res = cfg.analyze_with_language(&src, &shown_path, lang);

        let cg_block = match opts.mode {
            FormatMode::Ultra => cg.to_ultra_condensed(&graph),
            _ => cg.to_llm_string(&graph),
        };
        let cfg_block = match opts.mode {
            FormatMode::Ultra => cfg.to_ultra_condensed(&cfg_res),
            _ => cfg.to_llm_string(&cfg_res),
        };

        let combined = format!("{}\n{}", cg_block, cfg_block);
        blocks.push(format_block(opts.mode, &formatter, combined, 5000));
    }

    let mut out = String::new();
    out.push_str("# /phase3 Logic Flow\n");
    out.push_str(&format!("root: {}\n\n", root.display()));
    if read_errors > 0 {
        out.push_str(&format!("read_errors: {}\n\n", read_errors));
    }
    if blocks.is_empty() {
        out.push_str("(no analyzable files found)\n");
        return Ok(out);
    }
    out.push_str(&blocks.join("\n\n"));

    Ok(truncate_output(out, opts.max_output_chars))
}

pub fn phase4_critical_path(opts: &PhaseOptions) -> Result<String> {
    let formatter = TokenFormatter::new();
    let root = opts.root.clone();
    let files = collect_source_files(&root, opts.max_files)?;

    let mut cfg = MultiLangCFGAnalyzer::new();
    let mut hotspots: Vec<(usize, String)> = Vec::new();
    let mut read_errors: usize = 0;

    for file in &files {
        let lang = match ProgrammingLanguage::from_path(&file.to_string_lossy()) {
            Some(l) => l,
            None => continue,
        };
        let src = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => {
                read_errors += 1;
                continue;
            }
        };
        let shown_path = display_path(&root, file);

        let res = cfg.analyze_with_language(&src, &shown_path, lang);
        for m in res.function_metrics {
            let line = format!(
                "{}::{} cc={} cog={} L{}-{}",
                shown_path, m.function_name, m.cyclomatic_complexity, m.cognitive_complexity, m.line, m.end_line
            );
            hotspots.push((m.cyclomatic_complexity, line));
        }
    }

    hotspots.sort_by(|a, b| b.0.cmp(&a.0));
    hotspots.truncate(opts.top_n);

    let mut out = String::new();
    out.push_str("# /phase4 Critical Path\n");
    out.push_str(&format!("root: {}\n", root.display()));
    out.push_str(&format!("top_n: {}\n\n", opts.top_n));
    if read_errors > 0 {
        out.push_str(&format!("read_errors: {}\n\n", read_errors));
    }

    if hotspots.is_empty() {
        out.push_str("(no functions found)\n");
        return Ok(out);
    }

    for (_cc, line) in hotspots {
        out.push_str("- ");
        out.push_str(&formatter.truncate(&line, 400));
        out.push('\n');
    }

    Ok(truncate_output(out, opts.max_output_chars))
}

pub fn phase5_optimization_report(opts: &PhaseOptions) -> Result<String> {
    let formatter = TokenFormatter::new();
    let root = opts.root.clone();
    let files = collect_source_files(&root, opts.max_files)?;

    // Gather: language distribution + imports frequency + complexity hotspots
    let mut lang_counts: HashMap<String, usize> = HashMap::new();
    let mut dep_freq: HashMap<String, usize> = HashMap::new();
    let mut hotspots: Vec<(usize, String)> = Vec::new();

    let mut ast = MultiLangASTAnalyzer::new();
    let mut cfg = MultiLangCFGAnalyzer::new();

    for file in &files {
        let lang = match ProgrammingLanguage::from_path(&file.to_string_lossy()) {
            Some(l) => l,
            None => continue,
        };
        *lang_counts.entry(lang.display_name().to_string()).or_insert(0) += 1;

        let src = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let shown_path = display_path(&root, file);

        let a = ast.analyze_with_language(&src, &shown_path, lang);
        for imp in a.imports.iter().take(30) {
            let module = formatter.truncate(&imp.module, 50);
            *dep_freq.entry(module).or_insert(0) += 1;
        }

        let c = cfg.analyze_with_language(&src, &shown_path, lang);
        for m in c.function_metrics.into_iter().filter(|m| m.cyclomatic_complexity >= 10) {
            hotspots.push((
                m.cyclomatic_complexity,
                format!("{}::{} cc{} L{}", shown_path, m.function_name, m.cyclomatic_complexity, m.line),
            ));
        }
    }

    hotspots.sort_by(|a, b| b.0.cmp(&a.0));

    let mut out = String::new();
    out.push_str("# /phase5 Optimization Report\n");
    out.push_str(&format!("root: {}\n", root.display()));

    if !lang_counts.is_empty() {
        let mut langs: Vec<(String, usize)> = lang_counts.into_iter().collect();
        langs.sort_by(|a, b| b.1.cmp(&a.1));
        out.push_str("languages:");
        for (lang, count) in langs {
            out.push_str(&format!(" {}={}", lang, count));
        }
        out.push('\n');
    }

    if !dep_freq.is_empty() {
        let mut top: Vec<(String, usize)> = dep_freq.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        out.push_str("top_deps:");
        for (module, count) in top.into_iter().take(10) {
            out.push_str(&format!(" {}({})", module, count));
        }
        out.push('\n');
    }

    out.push('\n');
    out.push_str("recommendations:\n");

    if hotspots.is_empty() {
        out.push_str("- No cc>=10 hotspots detected in scanned files.\n");
    } else {
        out.push_str("- Prioritize refactors/tests around hotspots:\n");
        for (_cc, line) in hotspots.into_iter().take(10) {
            out.push_str("  - ");
            out.push_str(&formatter.truncate(&line, 220));
            out.push('\n');
        }
    }

    out.push_str("- For implementation tracks, start from /phase3 targets and keep context ultra.\n");

    Ok(truncate_output(out, opts.max_output_chars))
}
