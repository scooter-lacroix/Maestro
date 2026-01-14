//! Multi-Language Call Graph Analyzer (Layer 2)
//!
//! Builds function call relationships using tree-sitter for accurate parsing.
//! Supports all languages from the language module.

use crate::language::{
    child_by_field, find_all_nodes, get_language_config, node_text,
    MultiLanguageParser, ProgrammingLanguage,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Type of function call
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MultiLangCallType {
    Direct,      // foo()
    Method,      // obj.foo()
    Static,      // Class::foo() or Class.foo()
    Async,       // await foo()
    Chained,     // foo().bar()
    Constructor, // new Foo() or Foo()
}

/// A node in the call graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLangCallNode {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub file: String,
    pub line: usize,
    pub end_line: usize,
    pub is_method: bool,
    pub is_async: bool,
    pub class_name: Option<String>,
    pub callers: HashSet<String>,
    pub callees: HashSet<String>,
}

/// An edge in the call graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLangCallEdge {
    pub from_id: String,
    pub to_id: String,
    pub call_type: MultiLangCallType,
    pub line: usize,
    pub call_expr: String,
}

/// Call graph result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLangCallGraph {
    pub nodes: HashMap<String, MultiLangCallNode>,
    pub edges: Vec<MultiLangCallEdge>,
    pub entry_points: Vec<String>,
    pub leaf_functions: Vec<String>,
    pub file_path: String,
    pub language: String,
    pub external_calls: Vec<String>,
}

impl MultiLangCallGraph {
    pub fn new(file_path: &str, language: &str) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            entry_points: Vec::new(),
            leaf_functions: Vec::new(),
            file_path: file_path.to_string(),
            language: language.to_string(),
            external_calls: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: MultiLangCallNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: MultiLangCallEdge) {
        if let Some(from_node) = self.nodes.get_mut(&edge.from_id) {
            from_node.callees.insert(edge.to_id.clone());
        }
        if let Some(to_node) = self.nodes.get_mut(&edge.to_id) {
            to_node.callers.insert(edge.from_id.clone());
        }
        self.edges.push(edge);
    }

    pub fn find_entry_points(&mut self) {
        self.entry_points = self
            .nodes
            .iter()
            .filter(|(_, node)| {
                node.callers.is_empty()
                    || node.name == "main"
                    || node.name == "__init__"
                    || node.name.starts_with("test_")
                    || node.name.starts_with("Test")
            })
            .map(|(id, _)| id.clone())
            .collect();
    }

    pub fn find_leaf_functions(&mut self) {
        self.leaf_functions = self
            .nodes
            .iter()
            .filter(|(_, node)| node.callees.is_empty())
            .map(|(id, _)| id.clone())
            .collect();
    }

    /// Get summary statistics
    pub fn stats(&self) -> CallGraphStats {
        CallGraphStats {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            entry_points: self.entry_points.len(),
            leaf_functions: self.leaf_functions.len(),
            external_calls: self.external_calls.len(),
            max_depth: self.calculate_max_depth(),
        }
    }

    fn calculate_max_depth(&self) -> usize {
        let mut max_depth = 0;
        for entry in &self.entry_points {
            let depth = self.dfs_depth(entry, &mut HashSet::new());
            max_depth = max_depth.max(depth);
        }
        max_depth
    }

    fn dfs_depth(&self, node_id: &str, visited: &mut HashSet<String>) -> usize {
        if visited.contains(node_id) {
            return 0;
        }
        visited.insert(node_id.to_string());

        let mut max_child_depth = 0;
        if let Some(node) = self.nodes.get(node_id) {
            for callee in &node.callees {
                let depth = self.dfs_depth(callee, visited);
                max_child_depth = max_child_depth.max(depth);
            }
        }
        visited.remove(node_id);
        max_child_depth + 1
    }
}

/// Statistics about the call graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub entry_points: usize,
    pub leaf_functions: usize,
    pub external_calls: usize,
    pub max_depth: usize,
}

/// Multi-language call graph analyzer
pub struct MultiLangCallGraphAnalyzer {
    parser: MultiLanguageParser,
}

impl MultiLangCallGraphAnalyzer {
    pub fn new() -> Self {
        Self {
            parser: MultiLanguageParser::new(),
        }
    }

    /// Build call graph for a file with auto-detected language
    pub fn build_graph(&mut self, source: &str, path: &str) -> MultiLangCallGraph {
        let language = ProgrammingLanguage::from_path(path).unwrap_or(ProgrammingLanguage::Python);
        self.build_graph_with_language(source, path, language)
    }

    /// Build call graph with explicit language
    pub fn build_graph_with_language(
        &mut self,
        source: &str,
        path: &str,
        language: ProgrammingLanguage,
    ) -> MultiLangCallGraph {
        let mut graph = MultiLangCallGraph::new(path, language.display_name());

        let tree = match self.parser.parse(source, language) {
            Some(t) => t,
            None => return graph,
        };

        let root = tree.root_node();
        let config = get_language_config(language);

        // First pass: collect all function/method definitions
        let mut known_functions: HashMap<String, String> = HashMap::new();
        self.collect_functions(&mut graph, root, source, language, config.as_ref(), &mut known_functions);

        // Second pass: find all call expressions
        self.collect_calls(&mut graph, root, source, language, &known_functions);

        graph.find_entry_points();
        graph.find_leaf_functions();

        graph
    }

    fn collect_functions(
        &self,
        graph: &mut MultiLangCallGraph,
        root: tree_sitter::Node,
        source: &str,
        _language: ProgrammingLanguage,
        config: &dyn crate::language::LanguageConfig,
        known_functions: &mut HashMap<String, String>,
    ) {
        let func_nodes = find_all_nodes(root, config.function_node_types());

        for node in func_nodes.iter().take(100) {
            let line = node.start_position().row + 1;
            let end_line = node.end_position().row + 1;

            let name = match self.extract_function_name(*node, source) {
                Some(n) => n,
                None => continue,
            };

            // Check if inside a class
            let (is_method, class_name) = self.check_class_context(*node, source, config);
            
            let qualified_name = if let Some(ref cls) = class_name {
                format!("{}.{}", cls, name)
            } else {
                name.clone()
            };

            let node_id = format!("{}:{}:{}", graph.file_path, line, name);
            known_functions.insert(name.clone(), node_id.clone());
            known_functions.insert(qualified_name.clone(), node_id.clone());

            let is_async = self.check_is_async(*node, source);

            graph.add_node(MultiLangCallNode {
                id: node_id,
                name,
                qualified_name,
                file: graph.file_path.clone(),
                line,
                end_line,
                is_method,
                is_async,
                class_name,
                callers: HashSet::new(),
                callees: HashSet::new(),
            });
        }
    }

    fn extract_function_name(&self, node: tree_sitter::Node, source: &str) -> Option<String> {
        // Try name field
        if let Some(name_node) = child_by_field(node, "name") {
            return Some(node_text(name_node, source).to_string());
        }
        
        // Try declarator field (C/C++)
        if let Some(decl) = child_by_field(node, "declarator") {
            if let Some(name_node) = child_by_field(decl, "declarator") {
                return Some(node_text(name_node, source).to_string());
            }
            let text = node_text(decl, source);
            if let Some(paren) = text.find('(') {
                return Some(text[..paren].to_string());
            }
        }

        // Fallback: first identifier child
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                return Some(node_text(child, source).to_string());
            }
        }
        None
    }

    fn check_class_context(
        &self,
        node: tree_sitter::Node,
        source: &str,
        config: &dyn crate::language::LanguageConfig,
    ) -> (bool, Option<String>) {
        let mut parent = node.parent();
        while let Some(p) = parent {
            if config.class_node_types().contains(&p.kind()) {
                // Found class parent, get its name
                if let Some(name_node) = child_by_field(p, "name") {
                    return (true, Some(node_text(name_node, source).to_string()));
                }
                // Try type field (for struct)
                if let Some(name_node) = child_by_field(p, "type") {
                    return (true, Some(node_text(name_node, source).to_string()));
                }
                return (true, None);
            }
            parent = p.parent();
        }
        (false, None)
    }

    fn check_is_async(&self, node: tree_sitter::Node, source: &str) -> bool {
        let text = node_text(node, source);
        text.starts_with("async ") || text.contains("async fn")
    }

    fn collect_calls(
        &self,
        graph: &mut MultiLangCallGraph,
        root: tree_sitter::Node,
        source: &str,
        language: ProgrammingLanguage,
        known_functions: &HashMap<String, String>,
    ) {
        // Find call expressions based on language
        let call_types = match language {
            ProgrammingLanguage::Python => vec!["call"],
            ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
                vec!["call_expression", "new_expression"]
            }
            ProgrammingLanguage::Rust => vec!["call_expression", "macro_invocation"],
            ProgrammingLanguage::Go => vec!["call_expression"],
            ProgrammingLanguage::Java => vec!["method_invocation", "object_creation_expression"],
            ProgrammingLanguage::C | ProgrammingLanguage::Cpp => vec!["call_expression"],
        };

        let call_nodes = find_all_nodes(root, &call_types.iter().map(|s| *s).collect::<Vec<_>>());

        for call_node in call_nodes.iter().take(200) {
            let line = call_node.start_position().row + 1;
            let call_text = node_text(*call_node, source);

            // Extract function name being called
            let called_name = self.extract_call_target(*call_node, source, language);
            if called_name.is_empty() {
                continue;
            }

            // Find which function contains this call
            let caller_id = self.find_containing_function(*call_node, source, known_functions);
            if caller_id.is_none() {
                continue;
            }
            let caller_id = caller_id.unwrap();

            // Check if the called function is known
            if let Some(callee_id) = known_functions.get(&called_name) {
                if callee_id != &caller_id {
                    let call_type = self.determine_call_type(*call_node, source, language);
                    graph.add_edge(MultiLangCallEdge {
                        from_id: caller_id,
                        to_id: callee_id.clone(),
                        call_type,
                        line,
                        call_expr: call_text.chars().take(50).collect(),
                    });
                }
            } else {
                // External call
                if !graph.external_calls.contains(&called_name) && graph.external_calls.len() < 20 {
                    graph.external_calls.push(called_name);
                }
            }
        }
    }

    fn extract_call_target(
        &self,
        node: tree_sitter::Node,
        source: &str,
        _language: ProgrammingLanguage,
    ) -> String {
        // Try function field
        if let Some(func) = child_by_field(node, "function") {
            let text = node_text(func, source);
            // Handle method calls: obj.method -> method
            if let Some(dot_idx) = text.rfind('.') {
                return text[dot_idx + 1..].to_string();
            }
            return text.to_string();
        }

        // Try name field (Java)
        if let Some(name) = child_by_field(node, "name") {
            return node_text(name, source).to_string();
        }

        // Try method field (Java)
        if let Some(method) = child_by_field(node, "method") {
            return node_text(method, source).to_string();
        }

        // Fallback: look for identifier in children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "field_identifier" {
                return node_text(child, source).to_string();
            }
            // Handle member_expression
            if child.kind() == "member_expression" {
                if let Some(prop) = child_by_field(child, "property") {
                    return node_text(prop, source).to_string();
                }
            }
        }

        String::new()
    }

    fn find_containing_function(
        &self,
        node: tree_sitter::Node,
        source: &str,
        known_functions: &HashMap<String, String>,
    ) -> Option<String> {
        let mut parent = node.parent();
        while let Some(p) = parent {
            let kind = p.kind();
            // Check if this is a function definition
            if kind.contains("function") || kind.contains("method") {
                if let Some(name) = self.extract_function_name(p, source) {
                    return known_functions.get(&name).cloned();
                }
            }
            parent = p.parent();
        }
        None
    }

    fn determine_call_type(
        &self,
        node: tree_sitter::Node,
        source: &str,
        _language: ProgrammingLanguage,
    ) -> MultiLangCallType {
        let node_kind = node.kind();
        let text = node_text(node, source);

        // New expression
        if node_kind == "new_expression" || node_kind == "object_creation_expression" {
            return MultiLangCallType::Constructor;
        }

        // Await expression
        if text.starts_with("await ") {
            return MultiLangCallType::Async;
        }

        // Check for method call
        if let Some(func) = child_by_field(node, "function") {
            if func.kind() == "member_expression" || func.kind() == "attribute" {
                return MultiLangCallType::Method;
            }
        }

        // Check for static/scoped call (Rust :: or Java static)
        if text.contains("::") {
            return MultiLangCallType::Static;
        }

        // Check for chained call
        if text.matches('.').count() > 1 {
            return MultiLangCallType::Chained;
        }

        MultiLangCallType::Direct
    }

    /// Convert call graph to LLM-friendly string
    pub fn to_llm_string(&self, graph: &MultiLangCallGraph) -> String {
        let mut lines = Vec::new();
        let stats = graph.stats();

        lines.push(format!(
            "## Call Graph: {} ({})",
            graph.file_path, graph.language
        ));
        lines.push(format!(
            "# {} functions, {} calls, depth {}",
            stats.node_count, stats.edge_count, stats.max_depth
        ));

        // Entry points
        if !graph.entry_points.is_empty() {
            lines.push(String::new());
            lines.push("# Entry points".to_string());
            for id in graph.entry_points.iter().take(5) {
                if let Some(node) = graph.nodes.get(id) {
                    lines.push(format!("→ {} L{}", node.qualified_name, node.line));
                }
            }
        }

        // Call relationships (condensed)
        if !graph.edges.is_empty() {
            lines.push(String::new());
            lines.push("# Calls".to_string());
            let mut shown = 0;
            for edge in graph.edges.iter() {
                if shown >= 20 {
                    lines.push(format!("# +{} more calls", graph.edges.len() - 20));
                    break;
                }
                if let (Some(from), Some(to)) = (graph.nodes.get(&edge.from_id), graph.nodes.get(&edge.to_id)) {
                    lines.push(format!("{} → {} L{}", from.name, to.name, edge.line));
                    shown += 1;
                }
            }
        }

        // External calls
        if !graph.external_calls.is_empty() {
            lines.push(String::new());
            let ext_calls: Vec<&str> = graph.external_calls.iter().take(10).map(|s| s.as_str()).collect();
            lines.push(format!("# External: {}", ext_calls.join(", ")));
        }

        lines.join("\n")
    }
}

impl Default for MultiLangCallGraphAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_call_graph() {
        let mut analyzer = MultiLangCallGraphAnalyzer::new();
        let source = r#"
def helper():
    return 42

def main():
    result = helper()
    print(result)
"#;
        let graph = analyzer.build_graph(source, "test.py");
        
        assert_eq!(graph.language, "Python");
        assert!(graph.nodes.len() >= 2);
    }

    #[test]
    fn test_javascript_call_graph() {
        let mut analyzer = MultiLangCallGraphAnalyzer::new();
        let source = r#"
function helper() {
    return 42;
}

function main() {
    const result = helper();
    console.log(result);
}
"#;
        let graph = analyzer.build_graph(source, "test.js");
        
        assert_eq!(graph.language, "JavaScript");
        assert!(graph.nodes.len() >= 2);
    }

    #[test]
    fn test_rust_call_graph() {
        let mut analyzer = MultiLangCallGraphAnalyzer::new();
        let source = r#"
fn helper() -> i32 {
    42
}

fn main() {
    let result = helper();
    println!("{}", result);
}
"#;
        let graph = analyzer.build_graph(source, "test.rs");
        
        assert_eq!(graph.language, "Rust");
        assert!(graph.nodes.len() >= 2);
    }

    #[test]
    fn test_llm_output() {
        let mut analyzer = MultiLangCallGraphAnalyzer::new();
        let source = r#"
def a():
    b()

def b():
    c()

def c():
    pass
"#;
        let graph = analyzer.build_graph(source, "test.py");
        let output = analyzer.to_llm_string(&graph);
        
        assert!(output.contains("Call Graph"));
        assert!(output.contains("Python"));
    }
}
