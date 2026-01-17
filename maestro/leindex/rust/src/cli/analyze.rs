//! Analyze command implementation
//!
//! Performs code analysis on files using the multi-language analyzers.

use anyhow::Result;
use std::path::PathBuf;
use std::fs;

use leindex_analyzers::{
    MultiLangASTAnalyzer, MultiLangCallGraphAnalyzer,
    MultiLangCFGAnalyzer, MultiLangDFGAnalyzer, MultiLangSlicingAnalyzer,
    ProgrammingLanguage,
};

pub async fn run(
    path: PathBuf,
    format: String,
    language: Option<String>,
    analysis: String,
) -> Result<()> {
    // Read the file
    let source = fs::read_to_string(&path)?;
    let path_str = path.to_string_lossy();

    // Determine language
    let lang = if let Some(lang_str) = language {
        match lang_str.to_lowercase().as_str() {
            "python" | "py" => Some(ProgrammingLanguage::Python),
            "javascript" | "js" => Some(ProgrammingLanguage::JavaScript),
            "typescript" | "ts" => Some(ProgrammingLanguage::TypeScript),
            "rust" | "rs" => Some(ProgrammingLanguage::Rust),
            "go" => Some(ProgrammingLanguage::Go),
            "java" => Some(ProgrammingLanguage::Java),
            "c" => Some(ProgrammingLanguage::C),
            "cpp" | "c++" => Some(ProgrammingLanguage::Cpp),
            _ => {
                eprintln!("Unknown language: {}. Auto-detecting...", lang_str);
                ProgrammingLanguage::from_path(&path_str)
            }
        }
    } else {
        ProgrammingLanguage::from_path(&path_str)
    };

    let lang = lang.ok_or_else(|| anyhow::anyhow!("Could not detect language for: {}", path_str))?;

    println!("Analyzing {} ({})", path_str, lang.display_name());
    println!();

    // Run requested analysis
    match analysis.to_lowercase().as_str() {
        "ast" => run_ast_analysis(&source, &path_str, lang, &format),
        "callgraph" | "cg" => run_callgraph_analysis(&source, &path_str, lang, &format),
        "cfg" => run_cfg_analysis(&source, &path_str, lang, &format),
        "dfg" => run_dfg_analysis(&source, &path_str, lang, &format),
        "slicing" | "slice" => run_slicing_analysis(&source, &path_str, lang, &format),
        "all" => {
            println!("--- AST Analysis ---");
            run_ast_analysis(&source, &path_str, lang, &format);
            println!("\n--- CallGraph Analysis ---");
            run_callgraph_analysis(&source, &path_str, lang, &format);
            println!("\n--- CFG Analysis ---");
            run_cfg_analysis(&source, &path_str, lang, &format);
            println!("\n--- DFG Analysis ---");
            run_dfg_analysis(&source, &path_str, lang, &format);
            println!("\n--- Slicing Analysis ---");
            run_slicing_analysis(&source, &path_str, lang, &format);
        }
        _ => {
            eprintln!("Unknown analysis type: {}. Use: ast, callgraph, cfg, dfg, slicing, all", analysis);
        }
    }

    Ok(())
}

fn run_ast_analysis(source: &str, path: &str, lang: ProgrammingLanguage, format: &str) {
    let mut analyzer = MultiLangASTAnalyzer::new();
    let analysis = analyzer.analyze_with_language(source, path, lang);

    match format {
        "json" => println!("{}", serde_json::to_string_pretty(&analysis).unwrap_or_default()),
        "ultra" => println!("{}", analyzer.to_ultra_condensed(&analysis)),
        _ => println!("{}", analyzer.to_llm_string(&analysis)),
    }
}

fn run_callgraph_analysis(source: &str, path: &str, lang: ProgrammingLanguage, format: &str) {
    let mut analyzer = MultiLangCallGraphAnalyzer::new();
    let graph = analyzer.build_graph_with_language(source, path, lang);

    match format {
        "json" => println!("{}", serde_json::to_string_pretty(&graph).unwrap_or_default()),
        "ultra" => println!("{}", analyzer.to_ultra_condensed(&graph)),
        _ => println!("{}", analyzer.to_llm_string(&graph)),
    }
}

fn run_cfg_analysis(source: &str, path: &str, lang: ProgrammingLanguage, format: &str) {
    let mut analyzer = MultiLangCFGAnalyzer::new();
    let result = analyzer.analyze_with_language(source, path, lang);

    match format {
        "json" => println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default()),
        "ultra" => println!("{}", analyzer.to_ultra_condensed(&result)),
        _ => println!("{}", analyzer.to_llm_string(&result)),
    }
}

fn run_dfg_analysis(source: &str, path: &str, lang: ProgrammingLanguage, format: &str) {
    let mut analyzer = MultiLangDFGAnalyzer::new();
    let result = analyzer.analyze_with_language(source, path, lang);

    match format {
        "json" => println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default()),
        _ => println!("{}", analyzer.to_llm_string(&result)),
    }
}

fn run_slicing_analysis(source: &str, path: &str, lang: ProgrammingLanguage, format: &str) {
    let mut analyzer = MultiLangSlicingAnalyzer::new();
    let pdg = analyzer.build_pdg_with_language(source, path, lang);

    match format {
        "json" => println!("{}", serde_json::to_string_pretty(&pdg).unwrap_or_default()),
        _ => {
            println!("## PDG: {} ({})", path, lang.display_name());
            println!("# {} definitions, {} data deps, {} control deps",
                pdg.definitions.len(), pdg.data_deps.len(), pdg.control_deps.len());
        }
    }
}
