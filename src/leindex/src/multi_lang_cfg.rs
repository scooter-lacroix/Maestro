//! Multi-Language CFG Analyzer (Layer 3)
//!
//! Control flow complexity analysis using tree-sitter.
//! Calculates cyclomatic/cognitive complexity for all supported languages.

use crate::language::{
    find_all_nodes, get_language_config, node_text, MultiLanguageParser, ProgrammingLanguage,
};
use serde::{Deserialize, Serialize};
// Unused HashMap removed

/// Complexity metrics for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLangComplexityMetrics {
    pub function_name: String,
    pub line: usize,
    pub end_line: usize,
    pub cyclomatic_complexity: usize,
    pub cognitive_complexity: usize,
    pub decision_points: usize,
    pub loop_count: usize,
    pub branch_count: usize,
    pub exception_handlers: usize,
    pub max_nesting_depth: usize,
}

impl MultiLangComplexityMetrics {
    pub fn new(name: &str, line: usize, end_line: usize) -> Self {
        Self {
            function_name: name.to_string(),
            line,
            end_line,
            cyclomatic_complexity: 1, // Base
            cognitive_complexity: 0,
            decision_points: 0,
            loop_count: 0,
            branch_count: 0,
            exception_handlers: 0,
            max_nesting_depth: 0,
        }
    }

    pub fn complexity_rating(&self) -> &'static str {
        match self.cyclomatic_complexity {
            0..=5 => "low",
            6..=10 => "moderate",
            11..=20 => "high",
            _ => "very_high",
        }
    }
}

/// CFG analysis result for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLangCFGResult {
    pub file_path: String,
    pub language: String,
    pub function_metrics: Vec<MultiLangComplexityMetrics>,
    pub total_complexity: usize,
    pub average_complexity: f64,
    pub max_complexity: usize,
    pub high_complexity_functions: Vec<String>,
}

impl MultiLangCFGResult {
    pub fn new(file_path: &str, language: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            language: language.to_string(),
            function_metrics: Vec::new(),
            total_complexity: 0,
            average_complexity: 0.0,
            max_complexity: 0,
            high_complexity_functions: Vec::new(),
        }
    }

    pub fn finalize(&mut self) {
        self.total_complexity = self
            .function_metrics
            .iter()
            .map(|m| m.cyclomatic_complexity)
            .sum();

        self.max_complexity = self
            .function_metrics
            .iter()
            .map(|m| m.cyclomatic_complexity)
            .max()
            .unwrap_or(0);

        if !self.function_metrics.is_empty() {
            self.average_complexity =
                self.total_complexity as f64 / self.function_metrics.len() as f64;
        }

        self.high_complexity_functions = self
            .function_metrics
            .iter()
            .filter(|m| m.cyclomatic_complexity > 10)
            .map(|m| m.function_name.clone())
            .collect();
    }
}

/// Multi-language CFG analyzer
pub struct MultiLangCFGAnalyzer {
    parser: MultiLanguageParser,
}

impl MultiLangCFGAnalyzer {
    pub fn new() -> Self {
        Self {
            parser: MultiLanguageParser::new(),
        }
    }

    /// Analyze file with auto-detected language
    pub fn analyze(&mut self, source: &str, path: &str) -> MultiLangCFGResult {
        let language = ProgrammingLanguage::from_path(path).unwrap_or(ProgrammingLanguage::Python);
        self.analyze_with_language(source, path, language)
    }

    /// Analyze with explicit language
    pub fn analyze_with_language(
        &mut self,
        source: &str,
        path: &str,
        language: ProgrammingLanguage,
    ) -> MultiLangCFGResult {
        let mut result = MultiLangCFGResult::new(path, language.display_name());

        let tree = match self.parser.parse(source, language) {
            Some(t) => t,
            None => return result,
        };

        let root = tree.root_node();
        let config = get_language_config(language);

        // Find all function definitions
        let func_nodes = find_all_nodes(root, config.function_node_types());

        for func_node in func_nodes.iter().take(50) {
            if let Some(metrics) =
                self.analyze_function(*func_node, source, language, config.as_ref())
            {
                result.function_metrics.push(metrics);
            }
        }

        result.finalize();
        result
    }

    fn analyze_function(
        &self,
        node: tree_sitter::Node,
        source: &str,
        _language: ProgrammingLanguage,
        config: &dyn crate::language::LanguageConfig,
    ) -> Option<MultiLangComplexityMetrics> {
        let line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        let name = self.extract_function_name(node, source)?;
        let mut metrics = MultiLangComplexityMetrics::new(&name, line, end_line);

        // Analyze control flow within function body
        self.count_control_flow(node, source, config, &mut metrics, 0);

        Some(metrics)
    }

    fn extract_function_name(&self, node: tree_sitter::Node, source: &str) -> Option<String> {
        if let Some(name_node) = crate::language::child_by_field(node, "name") {
            return Some(node_text(name_node, source).to_string());
        }
        if let Some(decl) = crate::language::child_by_field(node, "declarator") {
            let text = node_text(decl, source);
            if let Some(paren) = text.find('(') {
                return Some(text[..paren].to_string());
            }
        }
        // Fallback: first identifier
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                return Some(node_text(child, source).to_string());
            }
        }
        None
    }

    fn count_control_flow(
        &self,
        node: tree_sitter::Node,
        source: &str,
        config: &dyn crate::language::LanguageConfig,
        metrics: &mut MultiLangComplexityMetrics,
        depth: usize,
    ) {
        let kind = node.kind();

        // Update max nesting
        metrics.max_nesting_depth = metrics.max_nesting_depth.max(depth);

        // Check for control flow constructs
        let is_decision = kind.contains("if") && !kind.contains("elif");
        let is_branch = kind.contains("elif") || kind.contains("else_if") || kind.contains("else");
        let is_loop = kind.contains("for") || kind.contains("while") || kind == "loop_expression";
        let is_exception = kind.contains("except") || kind.contains("catch");
        let is_case = kind.contains("case") || kind == "match_arm" || kind == "switch_case";

        if is_decision {
            metrics.cyclomatic_complexity += 1;
            metrics.cognitive_complexity += 1 + depth; // Cognitive adds nesting
            metrics.decision_points += 1;
        }

        if is_branch {
            metrics.branch_count += 1;
            if kind.contains("elif") || kind.contains("else_if") {
                metrics.cyclomatic_complexity += 1;
                metrics.cognitive_complexity += 1 + depth;
            }
        }

        if is_loop {
            metrics.cyclomatic_complexity += 1;
            metrics.cognitive_complexity += 1 + depth;
            metrics.loop_count += 1;
        }

        if is_exception {
            metrics.cyclomatic_complexity += 1;
            metrics.exception_handlers += 1;
        }

        if is_case {
            metrics.cyclomatic_complexity += 1;
            metrics.branch_count += 1;
        }

        // Boolean operators add complexity
        if kind == "boolean_operator" || kind == "binary_expression" {
            let text = node_text(node, source);
            if text.contains("&&") || text.contains("and") {
                metrics.cyclomatic_complexity += 1;
                metrics.cognitive_complexity += 1;
            }
            if text.contains("||") || text.contains("or") {
                metrics.cyclomatic_complexity += 1;
                metrics.cognitive_complexity += 1;
            }
        }

        // Ternary/conditional expressions
        if kind == "conditional_expression" || kind == "ternary_expression" {
            metrics.cyclomatic_complexity += 1;
            metrics.cognitive_complexity += 1 + depth;
        }

        // Recurse into children with updated depth
        let new_depth = if is_decision || is_loop || is_exception {
            depth + 1
        } else {
            depth
        };
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.count_control_flow(child, source, config, metrics, new_depth);
        }
    }

    /// Convert to LLM-friendly string
    pub fn to_llm_string(&self, result: &MultiLangCFGResult) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "## CFG: {} ({})",
            result.file_path, result.language
        ));
        lines.push(format!(
            "# {} functions, avg complexity {:.1}, max {}",
            result.function_metrics.len(),
            result.average_complexity,
            result.max_complexity
        ));

        // High complexity functions
        if !result.high_complexity_functions.is_empty() {
            lines.push(String::new());
            lines.push("# ⚠️ High complexity".to_string());
            for name in result.high_complexity_functions.iter().take(5) {
                if let Some(m) = result
                    .function_metrics
                    .iter()
                    .find(|m| &m.function_name == name)
                {
                    lines.push(format!(
                        "{}: cc={} cog={} L{}",
                        name, m.cyclomatic_complexity, m.cognitive_complexity, m.line
                    ));
                }
            }
        }

        // All functions (condensed)
        lines.push(String::new());
        lines.push("# Functions".to_string());
        for m in result.function_metrics.iter().take(20) {
            lines.push(format!(
                "{}: cc={} loops={} depth={} L{}",
                m.function_name, m.cyclomatic_complexity, m.loop_count, m.max_nesting_depth, m.line
            ));
        }
        if result.function_metrics.len() > 20 {
            lines.push(format!("# +{} more", result.function_metrics.len() - 20));
        }

        lines.join("\n")
    }

    /// Convert to ultra-condensed string for maximum token savings
    pub fn to_ultra_condensed(&self, result: &MultiLangCFGResult) -> String {
        let mut lines = Vec::new();

        lines.push(format!("## CFG {} ({})", result.file_path, result.language));
        lines.push(format!(
            "fn:{} avg:{:.1} max:{}",
            result.function_metrics.len(),
            result.average_complexity,
            result.max_complexity
        ));

        if !result.function_metrics.is_empty() {
            let mut metrics = result.function_metrics.clone();
            metrics.sort_by(|a, b| b.cyclomatic_complexity.cmp(&a.cyclomatic_complexity));
            let top: Vec<String> = metrics
                .iter()
                .take(10)
                .map(|m| {
                    format!(
                        "{}(cc{}@L{})",
                        m.function_name, m.cyclomatic_complexity, m.line
                    )
                })
                .collect();
            lines.push(format!("hot:{}", top.join(" ")));
        }

        lines.join("\n")
    }
}

impl Default for MultiLangCFGAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_complexity() {
        let mut analyzer = MultiLangCFGAnalyzer::new();
        let source = r#"
def simple():
    return 1

def complex(x):
    if x > 0:
        for i in range(x):
            if i % 2 == 0:
                print(i)
            else:
                print(-i)
    else:
        while x < 0:
            x += 1
    return x
"#;
        let result = analyzer.analyze(source, "test.py");

        assert_eq!(result.language, "Python");
        assert!(result.function_metrics.len() >= 2);

        // simple() should have low complexity
        let simple = result
            .function_metrics
            .iter()
            .find(|m| m.function_name == "simple");
        assert!(simple.is_some());
        assert!(simple.unwrap().cyclomatic_complexity <= 2);

        // complex() should have higher complexity
        let complex = result
            .function_metrics
            .iter()
            .find(|m| m.function_name == "complex");
        assert!(complex.is_some());
        assert!(complex.unwrap().cyclomatic_complexity > 3);
    }

    #[test]
    fn test_javascript_complexity() {
        let mut analyzer = MultiLangCFGAnalyzer::new();
        let source = r#"
function process(items) {
    for (const item of items) {
        if (item.active) {
            try {
                doSomething(item);
            } catch (e) {
                console.error(e);
            }
        }
    }
}
"#;
        let result = analyzer.analyze(source, "test.js");

        assert_eq!(result.language, "JavaScript");
        assert!(!result.function_metrics.is_empty());
    }

    #[test]
    fn test_rust_complexity() {
        let mut analyzer = MultiLangCFGAnalyzer::new();
        let source = r#"
fn process(items: &[Item]) -> Result<(), Error> {
    for item in items {
        match item.status {
            Status::Active => process_active(item)?,
            Status::Pending => process_pending(item)?,
            _ => {}
        }
    }
    Ok(())
}
"#;
        let result = analyzer.analyze(source, "test.rs");

        assert_eq!(result.language, "Rust");
        assert!(!result.function_metrics.is_empty());
    }

    #[test]
    fn test_llm_output() {
        let mut analyzer = MultiLangCFGAnalyzer::new();
        let source = r#"
def a():
    pass

def b():
    if True:
        pass
"#;
        let result = analyzer.analyze(source, "test.py");
        let output = analyzer.to_llm_string(&result);

        assert!(output.contains("CFG"));
        assert!(output.contains("Python"));
    }
}
