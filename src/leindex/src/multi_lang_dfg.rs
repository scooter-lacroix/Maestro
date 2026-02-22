//! Multi-Language DFG Analyzer (Layer 4)
//!
//! Data Flow Graph analysis using tree-sitter.
//! Tracks variable definitions, uses, and dependencies across all languages.

use crate::language::{
    child_by_field, find_all_nodes, get_language_config, node_text, MultiLanguageParser,
    ProgrammingLanguage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarDef {
    pub name: String,
    pub line: usize,
    pub definition_type: DefType,
    pub scope: String,
}

/// Type of definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefType {
    Assignment,
    Parameter,
    ForLoop,
    Import,
    FunctionDef,
    ClassDef,
}

/// Variable use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarUse {
    pub name: String,
    pub line: usize,
    pub use_type: UseType,
}

/// Type of use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseType {
    Read,
    Write,
    Call,
    Attribute,
}

/// Data flow information for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDataFlow {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    pub parameters: Vec<String>,
    pub local_vars: Vec<VarDef>,
    pub var_uses: Vec<VarUse>,
    pub external_refs: Vec<String>,
    pub returns: Vec<String>,
}

/// DFG analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLangDFGResult {
    pub file_path: String,
    pub language: String,
    pub global_vars: Vec<VarDef>,
    pub imports: Vec<String>,
    pub functions: Vec<FunctionDataFlow>,
    pub def_use_chains: Vec<DefUseChain>,
}

/// Definition-Use chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefUseChain {
    pub var_name: String,
    pub def_line: usize,
    pub use_lines: Vec<usize>,
}

impl MultiLangDFGResult {
    pub fn new(file_path: &str, language: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            language: language.to_string(),
            global_vars: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            def_use_chains: Vec::new(),
        }
    }
}

/// Multi-language DFG analyzer
pub struct MultiLangDFGAnalyzer {
    parser: MultiLanguageParser,
}

impl MultiLangDFGAnalyzer {
    pub fn new() -> Self {
        Self {
            parser: MultiLanguageParser::new(),
        }
    }

    /// Analyze file with auto-detected language
    pub fn analyze(&mut self, source: &str, path: &str) -> MultiLangDFGResult {
        let language = ProgrammingLanguage::from_path(path).unwrap_or(ProgrammingLanguage::Python);
        self.analyze_with_language(source, path, language)
    }

    /// Analyze with explicit language
    pub fn analyze_with_language(
        &mut self,
        source: &str,
        path: &str,
        language: ProgrammingLanguage,
    ) -> MultiLangDFGResult {
        let mut result = MultiLangDFGResult::new(path, language.display_name());

        let tree = match self.parser.parse(source, language) {
            Some(t) => t,
            None => return result,
        };

        let root = tree.root_node();
        let config = get_language_config(language);

        // Collect imports
        self.collect_imports(&mut result, root, source, language);

        // Collect global variables
        self.collect_global_vars(&mut result, root, source, config.as_ref());

        // Analyze each function
        let func_nodes = find_all_nodes(root, config.function_node_types());
        for func_node in func_nodes.iter().take(50) {
            if let Some(df) = self.analyze_function(*func_node, source, language, config.as_ref()) {
                result.functions.push(df);
            }
        }

        // Build def-use chains
        self.build_def_use_chains(&mut result);

        result
    }

    fn collect_imports(
        &self,
        result: &mut MultiLangDFGResult,
        root: tree_sitter::Node,
        source: &str,
        language: ProgrammingLanguage,
    ) {
        let import_types = match language {
            ProgrammingLanguage::Python => vec!["import_statement", "import_from_statement"],
            ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
                vec!["import_statement"]
            }
            ProgrammingLanguage::Rust => vec!["use_declaration"],
            ProgrammingLanguage::Go => vec!["import_declaration"],
            ProgrammingLanguage::Java => vec!["import_declaration"],
            ProgrammingLanguage::C | ProgrammingLanguage::Cpp => vec!["preproc_include"],
        };

        let import_nodes = find_all_nodes(root, &import_types);
        for node in import_nodes.iter().take(30) {
            let text = node_text(*node, source);
            result
                .imports
                .push(text.lines().next().unwrap_or(&text).to_string());
        }
    }

    fn collect_global_vars(
        &self,
        result: &mut MultiLangDFGResult,
        root: tree_sitter::Node,
        source: &str,
        config: &dyn crate::language::LanguageConfig,
    ) {
        // Look for top-level assignments/declarations
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            let kind = child.kind();
            let line = child.start_position().row + 1;

            // Skip function/class definitions
            if config.function_node_types().contains(&kind)
                || config.class_node_types().contains(&kind)
            {
                continue;
            }

            // For Python, expression_statement wraps assignments
            if kind == "expression_statement" {
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    if inner.kind().contains("assignment") {
                        if let Some(name) = self.extract_var_name(inner, source) {
                            result.global_vars.push(VarDef {
                                name,
                                line,
                                definition_type: DefType::Assignment,
                                scope: "global".to_string(),
                            });
                        }
                    }
                }
                continue;
            }

            // Check for variable declarations (other languages)
            if kind.contains("assignment") || kind.contains("declaration") {
                if let Some(name) = self.extract_var_name(child, source) {
                    result.global_vars.push(VarDef {
                        name,
                        line,
                        definition_type: DefType::Assignment,
                        scope: "global".to_string(),
                    });
                }
            }
        }
    }

    fn analyze_function(
        &self,
        node: tree_sitter::Node,
        source: &str,
        _language: ProgrammingLanguage,
        config: &dyn crate::language::LanguageConfig,
    ) -> Option<FunctionDataFlow> {
        let line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        let name = self.extract_function_name(node, source)?;
        let mut df = FunctionDataFlow {
            name,
            line,
            end_line,
            parameters: Vec::new(),
            local_vars: Vec::new(),
            var_uses: Vec::new(),
            external_refs: Vec::new(),
            returns: Vec::new(),
        };

        // Extract parameters
        if let Some(params) = child_by_field(node, "parameters") {
            self.extract_parameters(&mut df, params, source);
        }

        // Extract body
        if let Some(body) = child_by_field(node, "body") {
            self.analyze_body(&mut df, body, source, config);
        } else {
            // Try direct children for C-style functions
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind().contains("block") || child.kind().contains("body") {
                    self.analyze_body(&mut df, child, source, config);
                    break;
                }
            }
        }

        Some(df)
    }

    fn extract_function_name(&self, node: tree_sitter::Node, source: &str) -> Option<String> {
        if let Some(name_node) = child_by_field(node, "name") {
            return Some(node_text(name_node, source).to_string());
        }
        if let Some(decl) = child_by_field(node, "declarator") {
            let text = node_text(decl, source);
            if let Some(paren) = text.find('(') {
                return Some(text[..paren].to_string());
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                return Some(node_text(child, source).to_string());
            }
        }
        None
    }

    fn extract_parameters(&self, df: &mut FunctionDataFlow, node: tree_sitter::Node, source: &str) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "identifier" || kind == "typed_parameter" || kind == "parameter" {
                let text = node_text(child, source);
                // Extract just the parameter name
                let name = if let Some(colon) = text.find(':') {
                    text[..colon].trim().to_string()
                } else {
                    text.split_whitespace().next().unwrap_or(&text).to_string()
                };
                if !name.is_empty() && !name.starts_with('(') && !name.ends_with(')') {
                    df.parameters.push(name);
                }
            }
        }
    }

    fn analyze_body(
        &self,
        df: &mut FunctionDataFlow,
        node: tree_sitter::Node,
        source: &str,
        config: &dyn crate::language::LanguageConfig,
    ) {
        self.traverse_for_data_flow(df, node, source, config);
    }

    fn traverse_for_data_flow(
        &self,
        df: &mut FunctionDataFlow,
        node: tree_sitter::Node,
        source: &str,
        config: &dyn crate::language::LanguageConfig,
    ) {
        let kind = node.kind();
        let line = node.start_position().row + 1;

        // Check for assignments/definitions
        if kind.contains("assignment") || kind == "augmented_assignment" {
            if let Some(name) = self.extract_var_name(node, source) {
                df.local_vars.push(VarDef {
                    name: name.clone(),
                    line,
                    definition_type: DefType::Assignment,
                    scope: df.name.clone(),
                });
            }
        }

        // Check for variable declarations
        if kind.contains("declaration") && !kind.contains("function") && !kind.contains("class") {
            if let Some(name) = self.extract_var_name(node, source) {
                df.local_vars.push(VarDef {
                    name,
                    line,
                    definition_type: DefType::Assignment,
                    scope: df.name.clone(),
                });
            }
        }

        // Check for return statements
        if kind == "return_statement" {
            let text = node_text(node, source);
            let return_val = text.trim_start_matches("return").trim();
            if !return_val.is_empty() {
                // Extract identifiers from return
                self.extract_identifiers_from_text(return_val, &mut df.returns);
            }
        }

        // Check for identifier uses
        if kind == "identifier" {
            let name = node_text(node, source).to_string();
            // Determine if this is a read or write based on parent
            let use_type = if let Some(parent) = node.parent() {
                if parent.kind().contains("assignment")
                    && child_by_field(parent, "left")
                        .map(|n| n.id() == node.id())
                        .unwrap_or(false)
                {
                    UseType::Write
                } else if parent.kind().contains("call") {
                    UseType::Call
                } else if parent.kind().contains("attribute") {
                    UseType::Attribute
                } else {
                    UseType::Read
                }
            } else {
                UseType::Read
            };

            df.var_uses.push(VarUse {
                name,
                line,
                use_type,
            });
        }

        // Recurse
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_for_data_flow(df, child, source, config);
        }
    }

    fn extract_var_name(&self, node: tree_sitter::Node, source: &str) -> Option<String> {
        // Try left field (Python/JS assignment)
        if let Some(left) = child_by_field(node, "left") {
            return Some(node_text(left, source).to_string());
        }
        // Try name field
        if let Some(name) = child_by_field(node, "name") {
            return Some(node_text(name, source).to_string());
        }
        // Try declarator
        if let Some(decl) = child_by_field(node, "declarator") {
            return Some(node_text(decl, source).to_string());
        }
        None
    }

    fn extract_identifiers_from_text(&self, text: &str, out: &mut Vec<String>) {
        let mut current = String::new();
        for ch in text.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                current.push(ch);
            } else {
                if !current.is_empty()
                    && current
                        .chars()
                        .next()
                        .map(|c| c.is_alphabetic())
                        .unwrap_or(false)
                {
                    out.push(current.clone());
                }
                current.clear();
            }
        }
        if !current.is_empty()
            && current
                .chars()
                .next()
                .map(|c| c.is_alphabetic())
                .unwrap_or(false)
        {
            out.push(current);
        }
    }

    fn build_def_use_chains(&self, result: &mut MultiLangDFGResult) {
        let mut chains: HashMap<(String, usize), Vec<usize>> = HashMap::new();

        for func in &result.functions {
            // Build chains for local vars
            for def in &func.local_vars {
                let uses: Vec<usize> = func
                    .var_uses
                    .iter()
                    .filter(|u| u.name == def.name && u.line > def.line)
                    .map(|u| u.line)
                    .collect();
                if !uses.is_empty() {
                    chains.insert((def.name.clone(), def.line), uses);
                }
            }
        }

        result.def_use_chains = chains
            .into_iter()
            .map(|((name, def_line), use_lines)| DefUseChain {
                var_name: name,
                def_line,
                use_lines,
            })
            .collect();
    }

    /// Convert to LLM-friendly string
    pub fn to_llm_string(&self, result: &MultiLangDFGResult) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "## DFG: {} ({})",
            result.file_path, result.language
        ));

        let total_vars: usize = result.functions.iter().map(|f| f.local_vars.len()).sum();
        lines.push(format!(
            "# {} functions, {} globals, {} local vars",
            result.functions.len(),
            result.global_vars.len(),
            total_vars
        ));

        // Global vars
        if !result.global_vars.is_empty() {
            lines.push(String::new());
            lines.push("# Globals".to_string());
            for v in result.global_vars.iter().take(10) {
                lines.push(format!("{} L{}", v.name, v.line));
            }
        }

        // Functions with data flow
        lines.push(String::new());
        lines.push("# Functions".to_string());
        for f in result.functions.iter().take(15) {
            let params = f.parameters.join(",");
            let locals: Vec<&str> = f
                .local_vars
                .iter()
                .take(5)
                .map(|v| v.name.as_str())
                .collect();
            lines.push(format!(
                "{}({}) -> locals: {} L{}",
                f.name,
                params,
                locals.join(","),
                f.line
            ));
        }
        if result.functions.len() > 15 {
            lines.push(format!("# +{} more", result.functions.len() - 15));
        }

        // Def-use chains
        if !result.def_use_chains.is_empty() {
            lines.push(String::new());
            lines.push("# Def-Use chains".to_string());
            for chain in result.def_use_chains.iter().take(10) {
                lines.push(format!(
                    "{}: def@{} -> used@{:?}",
                    chain.var_name, chain.def_line, chain.use_lines
                ));
            }
        }

        lines.join("\n")
    }
}

impl Default for MultiLangDFGAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_dfg() {
        let mut analyzer = MultiLangDFGAnalyzer::new();
        let source = r#"
MAX_VALUE = 100

def process(data):
    result = []
    for item in data:
        if item < MAX_VALUE:
            result.append(item)
    return result
"#;
        let result = analyzer.analyze(source, "test.py");

        assert_eq!(result.language, "Python");
        assert!(!result.global_vars.is_empty());
        assert!(!result.functions.is_empty());

        let func = &result.functions[0];
        assert_eq!(func.name, "process");
        assert!(func.parameters.contains(&"data".to_string()));
    }

    #[test]
    fn test_javascript_dfg() {
        let mut analyzer = MultiLangDFGAnalyzer::new();
        let source = r#"
const MAX = 100;

function transform(items) {
    let results = [];
    for (const item of items) {
        results.push(item * 2);
    }
    return results;
}
"#;
        let result = analyzer.analyze(source, "test.js");

        assert_eq!(result.language, "JavaScript");
        assert!(!result.functions.is_empty());
    }

    #[test]
    fn test_rust_dfg() {
        let mut analyzer = MultiLangDFGAnalyzer::new();
        let source = r#"
const MAX: i32 = 100;

fn process(data: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in data {
        if item < MAX {
            result.push(item);
        }
    }
    result
}
"#;
        let result = analyzer.analyze(source, "test.rs");

        assert_eq!(result.language, "Rust");
        assert!(!result.functions.is_empty());
    }

    #[test]
    fn test_llm_output() {
        let mut analyzer = MultiLangDFGAnalyzer::new();
        let source = r#"
def foo(x):
    y = x + 1
    return y
"#;
        let result = analyzer.analyze(source, "test.py");
        let output = analyzer.to_llm_string(&result);

        assert!(output.contains("DFG"));
        assert!(output.contains("Python"));
    }
}
