//! CFG Analyzer (Layer 3)
//!
//! Control flow complexity analysis - calculates cyclomatic complexity,
//! identifies decision points, loops, and exception handling.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Types of nodes in the control flow graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CFGNodeType {
    Entry,
    Exit,
    BasicBlock,
    Condition,      // if/elif
    Loop,           // for/while
    Try,
    Except,
    Finally,
    With,
    Match,          // match statement (Python 3.10+)
}

/// A node in the control flow graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CFGNode {
    pub id: String,
    pub node_type: CFGNodeType,
    pub line: usize,
    pub condition: Option<String>,
    pub statements: Vec<String>,
    pub successors: HashSet<String>,
    pub predecessors: HashSet<String>,
}

/// Complexity metrics for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: usize,
    pub decision_points: usize,
    pub loop_count: usize,
    pub branch_count: usize,
    pub try_count: usize,
    pub except_count: usize,
    pub max_nesting_depth: usize,
    pub cognitive_complexity: usize,
}

impl ComplexityMetrics {
    pub fn new() -> Self {
        Self {
            cyclomatic_complexity: 1, // Base complexity is 1
            decision_points: 0,
            loop_count: 0,
            branch_count: 0,
            try_count: 0,
            except_count: 0,
            max_nesting_depth: 0,
            cognitive_complexity: 0,
        }
    }

    /// Get complexity rating as string
    pub fn complexity_score(&self) -> &'static str {
        match self.cyclomatic_complexity {
            0..=5 => "low",
            6..=10 => "moderate",
            11..=20 => "high",
            _ => "very_high",
        }
    }
}

impl Default for ComplexityMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Control flow graph for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFlowGraph {
    pub function_name: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub nodes: HashMap<String, CFGNode>,
    pub entry_node: Option<String>,
    pub exit_nodes: HashSet<String>,
    pub metrics: ComplexityMetrics,
}

impl ControlFlowGraph {
    pub fn new(function_name: &str, file_path: &str, start_line: usize) -> Self {
        Self {
            function_name: function_name.to_string(),
            file_path: file_path.to_string(),
            start_line,
            end_line: start_line,
            nodes: HashMap::new(),
            entry_node: None,
            exit_nodes: HashSet::new(),
            metrics: ComplexityMetrics::new(),
        }
    }
}

/// CFG analysis result for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CFGAnalysisResult {
    pub file_path: String,
    pub function_metrics: HashMap<String, ComplexityMetrics>,
    pub total_complexity: usize,
    pub average_complexity: f64,
    pub high_complexity_functions: Vec<String>,
}

/// CFG Analyzer
pub struct CFGAnalyzer {
    _node_counter: usize,
}

impl CFGAnalyzer {
    pub fn new() -> Self {
        Self { _node_counter: 0 }
    }

    /// Analyze a Python source file for control flow complexity
    pub fn analyze(&mut self, source: &str, file_path: &str) -> CFGAnalysisResult {
        let mut result = CFGAnalysisResult {
            file_path: file_path.to_string(),
            function_metrics: HashMap::new(),
            total_complexity: 0,
            average_complexity: 0.0,
            high_complexity_functions: Vec::new(),
        };

        let lines: Vec<&str> = source.lines().collect();

        // Find all function definitions and analyze each
        let mut function_starts: Vec<(String, usize, usize)> = Vec::new(); // (name, start_line, indent)

        for (line_num, raw_line) in lines.iter().enumerate() {
            let line = raw_line.trim();
            let indent = raw_line.len() - raw_line.trim_start().len();

            if line.starts_with("def ") || line.starts_with("async def ") {
                let is_async = line.starts_with("async def ");
                let rest = if is_async { &line[10..] } else { &line[4..] };
                if let Some(paren_pos) = rest.find('(') {
                    let func_name = rest[..paren_pos].trim().to_string();
                    function_starts.push((func_name, line_num + 1, indent));
                }
            }
        }

        // Analyze each function
        for (func_name, start_line, func_indent) in function_starts {
            // Find the function's end line
            let mut end_line = start_line;
            for (line_num, raw_line) in lines.iter().enumerate().skip(start_line) {
                let line = raw_line.trim();
                let indent = raw_line.len() - raw_line.trim_start().len();

                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Function ends when we see a line at same or lower indent
                if indent <= func_indent && line_num > start_line - 1 {
                    break;
                }
                end_line = line_num + 1;
            }

            // Extract function body and analyze
            let metrics = self.analyze_function_body(&lines, start_line, end_line, func_indent);
            result.function_metrics.insert(func_name.clone(), metrics.clone());

            if metrics.cyclomatic_complexity > 10 {
                result.high_complexity_functions.push(func_name);
            }
        }

        // Calculate totals
        result.total_complexity = result
            .function_metrics
            .values()
            .map(|m| m.cyclomatic_complexity)
            .sum();

        if !result.function_metrics.is_empty() {
            result.average_complexity =
                result.total_complexity as f64 / result.function_metrics.len() as f64;
        }

        result
    }

    /// Analyze a function body for complexity metrics
    fn analyze_function_body(
        &mut self,
        lines: &[&str],
        start_line: usize,
        end_line: usize,
        _base_indent: usize,
    ) -> ComplexityMetrics {
        let mut metrics = ComplexityMetrics::new();
        let mut nesting_depth = 0;
        let mut max_nesting = 0;
        let mut indent_stack: Vec<usize> = Vec::new();

        for line_idx in start_line..=end_line.min(lines.len()) {
            if line_idx == 0 || line_idx > lines.len() {
                continue;
            }
            let raw_line = lines[line_idx - 1];
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let indent = raw_line.len() - raw_line.trim_start().len();

            // Track nesting depth
            while !indent_stack.is_empty() && indent <= *indent_stack.last().unwrap() {
                indent_stack.pop();
                if nesting_depth > 0 {
                    nesting_depth -= 1;
                }
            }

            // Check for control flow statements
            if line.starts_with("if ") || line.starts_with("elif ") {
                metrics.decision_points += 1;
                metrics.branch_count += 1;
                metrics.cognitive_complexity += 1 + nesting_depth;
                indent_stack.push(indent);
                nesting_depth += 1;
            } else if line.starts_with("else:") {
                metrics.branch_count += 1;
                // else doesn't add to cyclomatic complexity but adds to cognitive
                metrics.cognitive_complexity += nesting_depth;
            } else if line.starts_with("for ") || line.starts_with("while ") {
                metrics.loop_count += 1;
                metrics.decision_points += 1;
                metrics.cognitive_complexity += 1 + nesting_depth;
                indent_stack.push(indent);
                nesting_depth += 1;
            } else if line.starts_with("try:") {
                metrics.try_count += 1;
                indent_stack.push(indent);
                nesting_depth += 1;
            } else if line.starts_with("except") {
                metrics.except_count += 1;
                metrics.decision_points += 1;
                metrics.cognitive_complexity += nesting_depth;
            } else if line.starts_with("finally:") {
                metrics.cognitive_complexity += nesting_depth;
            } else if line.starts_with("with ") {
                metrics.cognitive_complexity += 1;
                indent_stack.push(indent);
                nesting_depth += 1;
            } else if line.starts_with("match ") {
                metrics.decision_points += 1;
                metrics.cognitive_complexity += 1 + nesting_depth;
                indent_stack.push(indent);
                nesting_depth += 1;
            } else if line.starts_with("case ") {
                metrics.branch_count += 1;
                metrics.cognitive_complexity += nesting_depth;
            }

            // Check for logical operators (add to complexity)
            if line.contains(" and ") {
                let and_count = line.matches(" and ").count();
                metrics.decision_points += and_count;
                metrics.cognitive_complexity += and_count;
            }
            if line.contains(" or ") {
                let or_count = line.matches(" or ").count();
                metrics.decision_points += or_count;
                metrics.cognitive_complexity += or_count;
            }

            // Check for ternary expressions
            if line.contains(" if ") && line.contains(" else ") && !line.starts_with("if ") {
                metrics.decision_points += 1;
                metrics.cognitive_complexity += 1;
            }

            // Check for comprehensions with conditions
            if (line.contains("[") || line.contains("{"))
                && line.contains(" for ")
                && line.contains(" if ")
            {
                metrics.decision_points += 1;
                metrics.cognitive_complexity += 1;
            }

            max_nesting = max_nesting.max(nesting_depth);
        }

        metrics.max_nesting_depth = max_nesting;

        // Calculate cyclomatic complexity: E - N + 2P
        // Simplified: 1 + number of decision points
        metrics.cyclomatic_complexity = 1 + metrics.decision_points;

        metrics
    }

    /// Convert analysis result to LLM-friendly string
    pub fn to_llm_string(&self, result: &CFGAnalysisResult) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "# CFG Analysis: {} ({} functions)",
            result.file_path,
            result.function_metrics.len()
        ));

        if !result.function_metrics.is_empty() {
            lines.push(format!(
                "Total Complexity: {}, Average: {:.1}",
                result.total_complexity, result.average_complexity
            ));
        }

        // High complexity functions first
        if !result.high_complexity_functions.is_empty() {
            lines.push(String::new());
            lines.push("## High Complexity Functions (>10)".to_string());
            for func_name in &result.high_complexity_functions {
                if let Some(metrics) = result.function_metrics.get(func_name) {
                    lines.push(format!(
                        "  {} - CC:{} ({}), loops:{}, depth:{}",
                        func_name,
                        metrics.cyclomatic_complexity,
                        metrics.complexity_score(),
                        metrics.loop_count,
                        metrics.max_nesting_depth
                    ));
                }
            }
        }

        // All functions summary
        lines.push(String::new());
        lines.push("## Function Metrics".to_string());

        // Sort by complexity (descending)
        let mut sorted_funcs: Vec<_> = result.function_metrics.iter().collect();
        sorted_funcs.sort_by(|a, b| b.1.cyclomatic_complexity.cmp(&a.1.cyclomatic_complexity));

        for (func_name, metrics) in sorted_funcs.iter().take(20) {
            let complexity_indicator = match metrics.complexity_score() {
                "low" => "",
                "moderate" => "*",
                "high" => "**",
                "very_high" => "***",
                _ => "",
            };

            lines.push(format!(
                "  {}{} CC:{} loops:{} branches:{} depth:{}",
                func_name,
                complexity_indicator,
                metrics.cyclomatic_complexity,
                metrics.loop_count,
                metrics.branch_count,
                metrics.max_nesting_depth
            ));
        }

        if result.function_metrics.len() > 20 {
            lines.push(format!(
                "  ... and {} more functions",
                result.function_metrics.len() - 20
            ));
        }

        lines.join("\n")
    }
}

impl Default for CFGAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function_complexity() {
        let source = r#"
def simple():
    return 42
"#;
        let mut analyzer = CFGAnalyzer::new();
        let result = analyzer.analyze(source, "test.py");

        assert_eq!(result.function_metrics.len(), 1);
        let metrics = result.function_metrics.get("simple").unwrap();
        assert_eq!(metrics.cyclomatic_complexity, 1);
    }

    #[test]
    fn test_if_else_complexity() {
        let source = r#"
def branching(x):
    if x > 0:
        return 1
    elif x < 0:
        return -1
    else:
        return 0
"#;
        let mut analyzer = CFGAnalyzer::new();
        let result = analyzer.analyze(source, "test.py");

        let metrics = result.function_metrics.get("branching").unwrap();
        assert!(metrics.cyclomatic_complexity >= 3);
        assert!(metrics.branch_count >= 2);
    }

    #[test]
    fn test_loop_complexity() {
        let source = r#"
def looping(items):
    result = []
    for item in items:
        if item > 0:
            result.append(item)
    return result
"#;
        let mut analyzer = CFGAnalyzer::new();
        let result = analyzer.analyze(source, "test.py");

        let metrics = result.function_metrics.get("looping").unwrap();
        assert!(metrics.loop_count >= 1);
        assert!(metrics.max_nesting_depth >= 1);
    }

    #[test]
    fn test_nested_complexity() {
        let source = r#"
def nested(matrix):
    for row in matrix:
        for cell in row:
            if cell > 0:
                print(cell)
"#;
        let mut analyzer = CFGAnalyzer::new();
        let result = analyzer.analyze(source, "test.py");

        let metrics = result.function_metrics.get("nested").unwrap();
        assert!(metrics.max_nesting_depth >= 2);
    }
}
