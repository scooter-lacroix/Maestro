//! Call Graph Analyzer (Layer 2)
//!
//! Builds cross-file function relationships by tracking function calls.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::ast_analyzer::ASTAnalyzer;

/// Type of function call
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallType {
    Direct,
    Method,
    Async,
    Conditional,
}

/// A node in the call graph representing a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallNode {
    pub id: String,
    pub name: String,
    pub file: String,
    pub line: usize,
    pub is_method: bool,
    pub class_name: Option<String>,
    pub is_async: bool,
    pub callers: HashSet<String>,
    pub callees: HashSet<String>,
}

/// An edge in the call graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub from_id: String,
    pub to_id: String,
    pub call_type: CallType,
    pub line: usize,
}

/// Complete call graph for a file or project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    pub nodes: HashMap<String, CallNode>,
    pub edges: Vec<CallEdge>,
    pub entry_points: Vec<String>,
    pub leaf_functions: Vec<String>,
    pub file_path: String,
}

impl CallGraph {
    pub fn new(file_path: &str) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            entry_points: Vec::new(),
            leaf_functions: Vec::new(),
            file_path: file_path.to_string(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: CallNode) {
        let id = node.id.clone();
        self.nodes.insert(id, node);
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, from_id: &str, to_id: &str, call_type: CallType, line: usize) {
        // Update caller/callee relationships
        if let Some(from_node) = self.nodes.get_mut(from_id) {
            from_node.callees.insert(to_id.to_string());
        }
        if let Some(to_node) = self.nodes.get_mut(to_id) {
            to_node.callers.insert(from_id.to_string());
        }

        self.edges.push(CallEdge {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            call_type,
            line,
        });
    }

    /// Get all callers of a function
    pub fn get_callers(&self, function_id: &str) -> Vec<&CallNode> {
        self.edges
            .iter()
            .filter(|e| e.to_id == function_id)
            .filter_map(|e| self.nodes.get(&e.from_id))
            .collect()
    }

    /// Get all callees of a function
    pub fn get_callees(&self, function_id: &str) -> Vec<&CallNode> {
        self.edges
            .iter()
            .filter(|e| e.from_id == function_id)
            .filter_map(|e| self.nodes.get(&e.to_id))
            .collect()
    }

    /// Identify entry points (functions not called by anyone)
    pub fn find_entry_points(&mut self) {
        self.entry_points = self
            .nodes
            .iter()
            .filter(|(_, node)| {
                node.callers.is_empty()
                    || node.name == "main"
                    || node.name == "__init__"
                    || node.name.starts_with("test_")
            })
            .map(|(id, _)| id.clone())
            .collect();
    }

    /// Identify leaf functions (functions that don't call others)
    pub fn find_leaf_functions(&mut self) {
        self.leaf_functions = self
            .nodes
            .iter()
            .filter(|(_, node)| node.callees.is_empty())
            .map(|(id, _)| id.clone())
            .collect();
    }
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new("")
    }
}

/// Call Graph Analyzer
pub struct CallGraphAnalyzer {
    known_functions: HashMap<String, String>, // function_name -> node_id
}

impl CallGraphAnalyzer {
    pub fn new() -> Self {
        Self {
            known_functions: HashMap::new(),
        }
    }

    /// Build call graph for a single file
    pub fn build_file_graph(&mut self, source: &str, file_path: &str) -> CallGraph {
        let mut graph = CallGraph::new(file_path);
        self.known_functions.clear();

        // First, analyze the AST to get all functions
        let mut ast_analyzer = ASTAnalyzer::new();
        let ast = ast_analyzer.analyze(source, file_path);

        // Add all functions as nodes
        for func in &ast.functions {
            let node_id = format!("{}:{}", file_path, func.name);
            self.known_functions
                .insert(func.name.clone(), node_id.clone());

            graph.add_node(CallNode {
                id: node_id,
                name: func.name.clone(),
                file: file_path.to_string(),
                line: func.line,
                is_method: false,
                class_name: None,
                is_async: func.is_async,
                callers: HashSet::new(),
                callees: HashSet::new(),
            });
        }

        // Add class methods as nodes
        for cls in &ast.classes {
            for method in &cls.methods {
                let full_name = format!("{}.{}", cls.name, method.name);
                let node_id = format!("{}:{}", file_path, full_name);
                self.known_functions
                    .insert(full_name.clone(), node_id.clone());
                self.known_functions
                    .insert(method.name.clone(), node_id.clone());

                graph.add_node(CallNode {
                    id: node_id,
                    name: method.name.clone(),
                    file: file_path.to_string(),
                    line: method.line,
                    is_method: true,
                    class_name: Some(cls.name.clone()),
                    is_async: method.is_async,
                    callers: HashSet::new(),
                    callees: HashSet::new(),
                });
            }
        }

        // Extract function calls by parsing the source line by line
        self.extract_calls(&mut graph, source, file_path, &ast);

        // Find entry points and leaf functions
        graph.find_entry_points();
        graph.find_leaf_functions();

        graph
    }

    /// Extract function calls from source code
    fn extract_calls(
        &self,
        graph: &mut CallGraph,
        source: &str,
        _file_path: &str,
        _ast: &crate::ast_analyzer::ASTAnalysis,
    ) {
        let lines: Vec<&str> = source.lines().collect();

        // Track current function context
        let mut current_function: Option<String> = None;
        let mut function_indent = 0;

        for (line_num, raw_line) in lines.iter().enumerate() {
            let line_number = line_num + 1;
            let line = raw_line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let indent = raw_line.len() - raw_line.trim_start().len();

            // Check for function definition to track context
            if line.starts_with("def ") || line.starts_with("async def ") {
                let is_async = line.starts_with("async def ");
                let rest = if is_async { &line[10..] } else { &line[4..] };
                if let Some(paren_pos) = rest.find('(') {
                    let func_name = rest[..paren_pos].trim();
                    current_function = Some(func_name.to_string());
                    function_indent = indent;
                }
                continue;
            }

            // Check if we've exited the current function
            if let Some(ref _func) = current_function {
                if indent <= function_indent && !line.starts_with('@') {
                    current_function = None;
                }
            }

            // Extract function calls from this line
            if let Some(ref caller_name) = current_function {
                let caller_id = self.known_functions.get(caller_name);
                if caller_id.is_none() {
                    continue;
                }
                let caller_id = caller_id.unwrap().clone();

                // Find function calls in the line
                let calls = self.extract_calls_from_line(line);
                for call_name in calls {
                    // Check if this is a known function
                    if let Some(callee_id) = self.known_functions.get(&call_name) {
                        if callee_id != &caller_id {
                            // Avoid self-recursion duplicates
                            let call_type = if call_name.contains('.') {
                                CallType::Method
                            } else {
                                CallType::Direct
                            };
                            graph.add_edge(&caller_id, callee_id, call_type, line_number);
                        }
                    }
                }
            }
        }
    }

    /// Extract function call names from a line of code
    fn extract_calls_from_line(&self, line: &str) -> Vec<String> {
        let mut calls = Vec::new();
        let chars = line.chars();
        let mut current_name = String::new();
        let mut in_string = false;
        let mut string_char = ' ';

        for ch in chars {
            // Handle strings
            if (ch == '"' || ch == '\'') && !in_string {
                in_string = true;
                string_char = ch;
                current_name.clear();
                continue;
            }
            if in_string {
                if ch == string_char {
                    in_string = false;
                }
                continue;
            }

            // Build identifier (including dots for method calls)
            if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                current_name.push(ch);
            } else if ch == '(' && !current_name.is_empty() {
                // This is a function call
                // Skip built-in functions and keywords
                let call_name = current_name.clone();
                current_name.clear();

                if !self.is_builtin(&call_name) && !call_name.starts_with('.') {
                    // Handle method calls like self.method -> just method
                    let clean_name = if let Some(rest) = call_name.strip_prefix("self.") {
                        rest.to_string()
                    } else if call_name.contains('.') && !call_name.contains("self") {
                        // Keep the full qualified name for external calls
                        call_name
                    } else {
                        call_name
                    };
                    calls.push(clean_name);
                }
            } else {
                current_name.clear();
            }
        }

        calls
    }

    fn is_builtin(&self, name: &str) -> bool {
        let builtins = [
            "print",
            "len",
            "range",
            "str",
            "int",
            "float",
            "bool",
            "list",
            "dict",
            "set",
            "tuple",
            "type",
            "isinstance",
            "issubclass",
            "hasattr",
            "getattr",
            "setattr",
            "delattr",
            "callable",
            "iter",
            "next",
            "enumerate",
            "zip",
            "map",
            "filter",
            "sorted",
            "reversed",
            "all",
            "any",
            "sum",
            "min",
            "max",
            "abs",
            "round",
            "open",
            "input",
            "format",
            "repr",
            "id",
            "hash",
            "dir",
            "vars",
            "locals",
            "globals",
            "super",
            "property",
            "staticmethod",
            "classmethod",
            "object",
            "Exception",
            "ValueError",
            "TypeError",
            "KeyError",
            "IndexError",
            "AttributeError",
        ];
        builtins.contains(&name)
    }

    /// Convert call graph to LLM-friendly string
    pub fn to_llm_string(&self, graph: &CallGraph) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "# Call Graph: {} ({} functions, {} edges)",
            graph.file_path,
            graph.nodes.len(),
            graph.edges.len()
        ));

        // Entry points
        if !graph.entry_points.is_empty() {
            lines.push(String::new());
            lines.push("## Entry Points".to_string());
            for entry_id in graph.entry_points.iter().take(10) {
                if let Some(node) = graph.nodes.get(entry_id) {
                    let prefix = if node.is_async { "async " } else { "" };
                    let class_prefix = node
                        .class_name
                        .as_ref()
                        .map(|c| format!("{}.", c))
                        .unwrap_or_default();
                    lines.push(format!(
                        "  {}{}{} L{}",
                        prefix, class_prefix, node.name, node.line
                    ));
                }
            }
        }

        // Functions with their calls
        lines.push(String::new());
        lines.push("## Call Relationships".to_string());

        for node in graph.nodes.values() {
            if node.callees.is_empty() {
                continue;
            }

            let class_prefix = node
                .class_name
                .as_ref()
                .map(|c| format!("{}.", c))
                .unwrap_or_default();

            let callees: Vec<&str> = node
                .callees
                .iter()
                .filter_map(|callee_id| graph.nodes.get(callee_id).map(|n| n.name.as_str()))
                .take(5)
                .collect();

            if !callees.is_empty() {
                lines.push(format!(
                    "  {}{} -> [{}]",
                    class_prefix,
                    node.name,
                    callees.join(", ")
                ));
            }
        }

        // Leaf functions
        if !graph.leaf_functions.is_empty() {
            lines.push(String::new());
            lines.push("## Leaf Functions (no outgoing calls)".to_string());
            let leaf_names: Vec<&str> = graph
                .leaf_functions
                .iter()
                .filter_map(|id| graph.nodes.get(id).map(|n| n.name.as_str()))
                .take(10)
                .collect();
            lines.push(format!("  {}", leaf_names.join(", ")));
        }

        lines.join("\n")
    }
}

impl Default for CallGraphAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_simple_graph() {
        let source = r#"
def caller():
    callee()

def callee():
    pass
"#;
        let mut analyzer = CallGraphAnalyzer::new();
        let graph = analyzer.build_file_graph(source, "test.py");

        assert_eq!(graph.nodes.len(), 2);
        assert!(!graph.edges.is_empty());
    }

    #[test]
    fn test_entry_points() {
        let source = r#"
def main():
    helper()

def helper():
    pass
"#;
        let mut analyzer = CallGraphAnalyzer::new();
        let graph = analyzer.build_file_graph(source, "test.py");

        assert!(graph.entry_points.iter().any(|id| id.contains("main")));
    }
}
