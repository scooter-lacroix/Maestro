//! Multi-Language Program Slicing (Layer 5)
//!
//! Program slicing using tree-sitter for dependency analysis.
//! Supports backward and forward slicing across all languages.

use crate::language::{
    child_by_field, find_all_nodes, get_language_config, node_text,
    MultiLanguageParser, ProgrammingLanguage,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// A slice point (variable at a specific location)
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct SlicePoint {
    pub variable: String,
    pub line: usize,
}

/// Dependency between program points
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: SlicePoint,
    pub to: SlicePoint,
    pub dep_type: DependencyType,
}

/// Type of dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    Data,    // Data dependency (def-use)
    Control, // Control dependency (in branch)
}

/// A program slice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramSlice {
    pub criterion: SlicePoint,
    pub slice_type: SliceType,
    pub lines: Vec<usize>,
    pub variables: Vec<String>,
    pub dependencies: Vec<Dependency>,
}

/// Type of slice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SliceType {
    Backward,
    Forward,
}

/// Program Dependence Graph for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLangPDG {
    pub file_path: String,
    pub language: String,
    pub definitions: HashMap<String, Vec<usize>>,  // var -> def lines
    pub uses: HashMap<String, Vec<usize>>,          // var -> use lines
    pub data_deps: Vec<Dependency>,
    pub control_deps: Vec<Dependency>,
}

impl MultiLangPDG {
    pub fn new(file_path: &str, language: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            language: language.to_string(),
            definitions: HashMap::new(),
            uses: HashMap::new(),
            data_deps: Vec::new(),
            control_deps: Vec::new(),
        }
    }
}

/// Multi-language program slicing analyzer
pub struct MultiLangSlicingAnalyzer {
    parser: MultiLanguageParser,
}

impl MultiLangSlicingAnalyzer {
    pub fn new() -> Self {
        Self {
            parser: MultiLanguageParser::new(),
        }
    }

    /// Build PDG for a file
    pub fn build_pdg(&mut self, source: &str, path: &str) -> MultiLangPDG {
        let language = ProgrammingLanguage::from_path(path).unwrap_or(ProgrammingLanguage::Python);
        self.build_pdg_with_language(source, path, language)
    }

    /// Build PDG with explicit language
    pub fn build_pdg_with_language(
        &mut self,
        source: &str,
        path: &str,
        language: ProgrammingLanguage,
    ) -> MultiLangPDG {
        let mut pdg = MultiLangPDG::new(path, language.display_name());

        let tree = match self.parser.parse(source, language) {
            Some(t) => t,
            None => return pdg,
        };

        let root = tree.root_node();
        let config = get_language_config(language);

        // Collect definitions and uses
        self.collect_defs_and_uses(&mut pdg, root, source, config.as_ref());

        // Build data dependencies
        self.build_data_deps(&mut pdg);

        // Build control dependencies
        self.build_control_deps(&mut pdg, root, source, config.as_ref());

        pdg
    }

    fn collect_defs_and_uses(
        &self,
        pdg: &mut MultiLangPDG,
        node: tree_sitter::Node,
        source: &str,
        config: &dyn crate::language::LanguageConfig,
    ) {
        self.traverse_for_deps(pdg, node, source, config, false);
    }

    fn traverse_for_deps(
        &self,
        pdg: &mut MultiLangPDG,
        node: tree_sitter::Node,
        source: &str,
        config: &dyn crate::language::LanguageConfig,
        in_control_structure: bool,
    ) {
        let kind = node.kind();
        let line = node.start_position().row + 1;

        // Check for assignments (definitions)
        if kind.contains("assignment") || kind == "augmented_assignment" {
            if let Some(left) = child_by_field(node, "left") {
                let var_name = node_text(left, source).to_string();
                pdg.definitions.entry(var_name.clone()).or_default().push(line);

                // Right side contains uses
                if let Some(right) = child_by_field(node, "right") {
                    self.collect_identifiers_as_uses(pdg, right, source, line);
                }
            }
        }

        // Check for variable declarations
        if kind.contains("declaration") && !kind.contains("function") && !kind.contains("class") {
            if let Some(decl) = child_by_field(node, "declarator") {
                let var_name = node_text(decl, source).to_string();
                pdg.definitions.entry(var_name).or_default().push(line);
            }
            if let Some(name) = child_by_field(node, "name") {
                let var_name = node_text(name, source).to_string();
                pdg.definitions.entry(var_name).or_default().push(line);
            }
        }

        // Check for identifier uses
        if kind == "identifier" {
            let var_name = node_text(node, source).to_string();
            // Skip if this is the left side of an assignment
            if let Some(parent) = node.parent() {
                let is_def = parent.kind().contains("assignment")
                    && child_by_field(parent, "left")
                        .map(|n| n.id() == node.id())
                        .unwrap_or(false);
                if !is_def {
                    pdg.uses.entry(var_name).or_default().push(line);
                }
            } else {
                pdg.uses.entry(var_name).or_default().push(line);
            }
        }

        // Recurse
        let new_in_control = in_control_structure
            || kind.contains("if")
            || kind.contains("for")
            || kind.contains("while");

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_for_deps(pdg, child, source, config, new_in_control);
        }
    }

    fn collect_identifiers_as_uses(
        &self,
        pdg: &mut MultiLangPDG,
        node: tree_sitter::Node,
        source: &str,
        line: usize,
    ) {
        if node.kind() == "identifier" {
            let var_name = node_text(node, source).to_string();
            pdg.uses.entry(var_name).or_default().push(line);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifiers_as_uses(pdg, child, source, line);
        }
    }

    fn build_data_deps(&self, pdg: &mut MultiLangPDG) {
        // For each use, find the reaching definition
        for (var, use_lines) in &pdg.uses {
            if let Some(def_lines) = pdg.definitions.get(var) {
                for &use_line in use_lines {
                    // Find the most recent definition before this use
                    let reaching_def = def_lines
                        .iter()
                        .filter(|&&d| d < use_line)
                        .max();

                    if let Some(&def_line) = reaching_def {
                        pdg.data_deps.push(Dependency {
                            from: SlicePoint {
                                variable: var.clone(),
                                line: def_line,
                            },
                            to: SlicePoint {
                                variable: var.clone(),
                                line: use_line,
                            },
                            dep_type: DependencyType::Data,
                        });
                    }
                }
            }
        }
    }

    fn build_control_deps(
        &self,
        pdg: &mut MultiLangPDG,
        root: tree_sitter::Node,
        source: &str,
        _config: &dyn crate::language::LanguageConfig,
    ) {
        // Find control structures and add control dependencies
        let control_types: &[&str] = &[
            "if_statement", "for_statement", "while_statement",
            "if_expression", "for_expression", "while_expression",
            "match_expression", "switch_statement",
        ];

        let control_nodes = find_all_nodes(root, control_types);

        for ctrl in control_nodes.iter().take(50) {
            let ctrl_line = ctrl.start_position().row + 1;

            // Get condition variable if any
            let condition_var = if let Some(cond) = child_by_field(*ctrl, "condition") {
                self.extract_first_identifier(cond, source)
            } else {
                None
            };

            // All lines in the body are control-dependent on the condition
            if let Some(body) = child_by_field(*ctrl, "body") {
                let body_start = body.start_position().row + 1;
                let body_end = body.end_position().row + 1;

                for line in body_start..=body_end {
                    if let Some(ref var) = condition_var {
                        pdg.control_deps.push(Dependency {
                            from: SlicePoint {
                                variable: var.clone(),
                                line: ctrl_line,
                            },
                            to: SlicePoint {
                                variable: "_control".to_string(),
                                line,
                            },
                            dep_type: DependencyType::Control,
                        });
                    }
                }
            }
        }
    }

    fn extract_first_identifier(&self, node: tree_sitter::Node, source: &str) -> Option<String> {
        if node.kind() == "identifier" {
            return Some(node_text(node, source).to_string());
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(id) = self.extract_first_identifier(child, source) {
                return Some(id);
            }
        }
        None
    }

    /// Compute backward slice from a criterion
    pub fn backward_slice(&self, pdg: &MultiLangPDG, criterion: SlicePoint) -> ProgramSlice {
        let mut slice = ProgramSlice {
            criterion: criterion.clone(),
            slice_type: SliceType::Backward,
            lines: Vec::new(),
            variables: Vec::new(),
            dependencies: Vec::new(),
        };

        let mut visited: HashSet<usize> = HashSet::new();
        let mut worklist: VecDeque<usize> = VecDeque::new();
        let mut relevant_vars: HashSet<String> = HashSet::new();

        worklist.push_back(criterion.line);
        relevant_vars.insert(criterion.variable.clone());

        while let Some(line) = worklist.pop_front() {
            if visited.contains(&line) {
                continue;
            }
            visited.insert(line);
            slice.lines.push(line);

            // Find dependencies that affect this line
            for dep in pdg.data_deps.iter().chain(pdg.control_deps.iter()) {
                if dep.to.line == line || (relevant_vars.contains(&dep.to.variable) && dep.to.line <= line) {
                    if !visited.contains(&dep.from.line) {
                        worklist.push_back(dep.from.line);
                        relevant_vars.insert(dep.from.variable.clone());
                        slice.dependencies.push(dep.clone());
                    }
                }
            }
        }

        slice.lines.sort();
        slice.lines.dedup();
        slice.variables = relevant_vars.into_iter().collect();
        slice
    }

    /// Compute forward slice from a criterion
    pub fn forward_slice(&self, pdg: &MultiLangPDG, criterion: SlicePoint) -> ProgramSlice {
        let mut slice = ProgramSlice {
            criterion: criterion.clone(),
            slice_type: SliceType::Forward,
            lines: Vec::new(),
            variables: Vec::new(),
            dependencies: Vec::new(),
        };

        let mut visited: HashSet<usize> = HashSet::new();
        let mut worklist: VecDeque<usize> = VecDeque::new();
        let mut relevant_vars: HashSet<String> = HashSet::new();

        worklist.push_back(criterion.line);
        relevant_vars.insert(criterion.variable.clone());

        while let Some(line) = worklist.pop_front() {
            if visited.contains(&line) {
                continue;
            }
            visited.insert(line);
            slice.lines.push(line);

            // Find dependencies from this line
            for dep in pdg.data_deps.iter().chain(pdg.control_deps.iter()) {
                if dep.from.line == line || (relevant_vars.contains(&dep.from.variable) && dep.from.line >= line) {
                    if !visited.contains(&dep.to.line) {
                        worklist.push_back(dep.to.line);
                        relevant_vars.insert(dep.to.variable.clone());
                        slice.dependencies.push(dep.clone());
                    }
                }
            }
        }

        slice.lines.sort();
        slice.lines.dedup();
        slice.variables = relevant_vars.into_iter().collect();
        slice
    }

    /// Convert slice to LLM-friendly string
    pub fn to_llm_string(&self, slice: &ProgramSlice) -> String {
        let mut lines = Vec::new();

        let slice_type = match slice.slice_type {
            SliceType::Backward => "Backward",
            SliceType::Forward => "Forward",
        };

        lines.push(format!(
            "## {} Slice: {} @ L{}",
            slice_type, slice.criterion.variable, slice.criterion.line
        ));
        lines.push(format!(
            "# {} lines, {} vars",
            slice.lines.len(),
            slice.variables.len()
        ));

        // Lines in slice
        lines.push(String::new());
        lines.push("# Lines in slice".to_string());
        let line_str: Vec<String> = slice.lines.iter().take(20).map(|l| l.to_string()).collect();
        lines.push(format!("L{}", line_str.join(", L")));

        // Variables
        if !slice.variables.is_empty() {
            lines.push(String::new());
            lines.push(format!("# Vars: {}", slice.variables.join(", ")));
        }

        lines.join("\n")
    }
}

impl Default for MultiLangSlicingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_pdg() {
        let mut analyzer = MultiLangSlicingAnalyzer::new();
        let source = r#"
x = 10
y = x + 5
z = y * 2
print(z)
"#;
        let pdg = analyzer.build_pdg(source, "test.py");
        
        assert_eq!(pdg.language, "Python");
        assert!(!pdg.definitions.is_empty());
        assert!(!pdg.uses.is_empty());
    }

    #[test]
    fn test_backward_slice() {
        let mut analyzer = MultiLangSlicingAnalyzer::new();
        let source = r#"
x = 10
y = x + 5
z = y * 2
w = 100
print(z)
"#;
        let pdg = analyzer.build_pdg(source, "test.py");
        let slice = analyzer.backward_slice(&pdg, SlicePoint {
            variable: "z".to_string(),
            line: 4,
        });
        
        assert_eq!(slice.slice_type, SliceType::Backward);
        assert!(!slice.lines.is_empty());
        // w = 100 should not be in the slice
    }

    #[test]
    fn test_forward_slice() {
        let mut analyzer = MultiLangSlicingAnalyzer::new();
        let source = r#"
x = 10
y = x + 5
z = y * 2
print(z)
"#;
        let pdg = analyzer.build_pdg(source, "test.py");
        let slice = analyzer.forward_slice(&pdg, SlicePoint {
            variable: "x".to_string(),
            line: 2,
        });
        
        assert_eq!(slice.slice_type, SliceType::Forward);
        assert!(!slice.lines.is_empty());
    }

    #[test]
    fn test_llm_output() {
        let mut analyzer = MultiLangSlicingAnalyzer::new();
        let source = r#"
x = 10
y = x + 5
"#;
        let pdg = analyzer.build_pdg(source, "test.py");
        let slice = analyzer.backward_slice(&pdg, SlicePoint {
            variable: "y".to_string(),
            line: 3,
        });
        let output = analyzer.to_llm_string(&slice);
        
        assert!(output.contains("Backward Slice"));
    }
}
