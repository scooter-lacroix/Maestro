//! DFG Analyzer (Layer 4)
//!
//! Data flow analysis: tracks variable definitions, uses, and dependencies.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Types of variable actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VarAction {
    Define,
    Read,
    Modify,
    Delete,
}

/// A single variable access event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableAccess {
    pub name: String,
    pub action: VarAction,
    pub line: usize,
    pub scope: String,
    pub context: Option<String>,
}

/// Complete information about a variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableInfo {
    pub name: String,
    pub defining_line: usize,
    pub defining_scope: String,
    pub type_hint: Option<String>,
    pub is_parameter: bool,
    pub is_global: bool,
    pub is_nonlocal: bool,
    pub accesses: Vec<VariableAccess>,
}

impl VariableInfo {
    pub fn new(name: &str, defining_line: usize, scope: &str) -> Self {
        Self {
            name: name.to_string(),
            defining_line,
            defining_scope: scope.to_string(),
            type_hint: None,
            is_parameter: false,
            is_global: false,
            is_nonlocal: false,
            accesses: Vec::new(),
        }
    }

    /// Count definitions
    pub fn def_count(&self) -> usize {
        self.accesses
            .iter()
            .filter(|a| a.action == VarAction::Define)
            .count()
    }

    /// Count reads
    pub fn read_count(&self) -> usize {
        self.accesses
            .iter()
            .filter(|a| a.action == VarAction::Read)
            .count()
    }

    /// Count modifications
    pub fn modify_count(&self) -> usize {
        self.accesses
            .iter()
            .filter(|a| a.action == VarAction::Modify)
            .count()
    }

    /// Get all lines where this variable is used
    pub fn use_lines(&self) -> Vec<usize> {
        self.accesses
            .iter()
            .filter(|a| a.action == VarAction::Read || a.action == VarAction::Modify)
            .map(|a| a.line)
            .collect()
    }
}

/// Data flow graph for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDataFlow {
    pub function_name: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parameters: Vec<String>,
    pub variables: HashMap<String, VariableInfo>,
    pub returns: Vec<String>,
    pub globals_used: HashSet<String>,
    pub nonlocals_used: HashSet<String>,
}

impl FunctionDataFlow {
    pub fn new(function_name: &str, file_path: &str, start_line: usize) -> Self {
        Self {
            function_name: function_name.to_string(),
            file_path: file_path.to_string(),
            start_line,
            end_line: start_line,
            parameters: Vec::new(),
            variables: HashMap::new(),
            returns: Vec::new(),
            globals_used: HashSet::new(),
            nonlocals_used: HashSet::new(),
        }
    }
}

/// DFG analysis result for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DFGAnalysisResult {
    pub file_path: String,
    pub functions: HashMap<String, FunctionDataFlow>,
    pub global_variables: HashMap<String, VariableInfo>,
}

/// DFG Analyzer
pub struct DFGAnalyzer;

impl DFGAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a Python source file for data flow
    pub fn analyze(&mut self, source: &str, file_path: &str) -> DFGAnalysisResult {
        let mut result = DFGAnalysisResult {
            file_path: file_path.to_string(),
            functions: HashMap::new(),
            global_variables: HashMap::new(),
        };

        let lines: Vec<&str> = source.lines().collect();

        // Find all function definitions
        let mut function_ranges: Vec<(String, usize, usize, usize)> = Vec::new(); // (name, start, end, indent)

        for (line_num, raw_line) in lines.iter().enumerate() {
            let line = raw_line.trim();
            let indent = raw_line.len() - raw_line.trim_start().len();

            if line.starts_with("def ") || line.starts_with("async def ") {
                let is_async = line.starts_with("async def ");
                let rest = if is_async { &line[10..] } else { &line[4..] };
                if let Some(paren_pos) = rest.find('(') {
                    let func_name = rest[..paren_pos].trim().to_string();

                    // Find function end
                    let mut end_line = line_num + 1;
                    for (idx, next_line) in lines.iter().enumerate().skip(line_num + 1) {
                        let next_trimmed = next_line.trim();
                        let next_indent = next_line.len() - next_line.trim_start().len();

                        if next_trimmed.is_empty() || next_trimmed.starts_with('#') {
                            continue;
                        }

                        if next_indent <= indent {
                            break;
                        }
                        end_line = idx + 1;
                    }

                    function_ranges.push((func_name, line_num + 1, end_line, indent));
                }
            }
        }

        // Analyze each function
        for (func_name, start_line, end_line, _func_indent) in function_ranges {
            let dfg = self.analyze_function(&lines, &func_name, file_path, start_line, end_line);
            result.functions.insert(func_name, dfg);
        }

        // Analyze global variables
        self.analyze_globals(&lines, &mut result);

        result
    }

    /// Analyze a function for data flow
    fn analyze_function(
        &self,
        lines: &[&str],
        func_name: &str,
        file_path: &str,
        start_line: usize,
        end_line: usize,
    ) -> FunctionDataFlow {
        let mut dfg = FunctionDataFlow::new(func_name, file_path, start_line);
        dfg.end_line = end_line;

        // Parse function signature for parameters
        if start_line > 0 && start_line <= lines.len() {
            let def_line = lines[start_line - 1].trim();
            if let Some(paren_start) = def_line.find('(') {
                if let Some(paren_end) = def_line.rfind(')') {
                    let args_str = &def_line[paren_start + 1..paren_end];
                    self.parse_parameters(args_str, &mut dfg);
                }
            }
        }

        // Track defined variables in function scope
        let mut defined_vars: HashSet<String> = dfg.parameters.iter().cloned().collect();

        // Analyze function body
        for line_idx in start_line..=end_line.min(lines.len()) {
            if line_idx == 0 || line_idx > lines.len() {
                continue;
            }
            let line = lines[line_idx - 1].trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check for global/nonlocal declarations
            if line.starts_with("global ") {
                let names = line[7..].split(',');
                for name in names {
                    let name = name.trim();
                    dfg.globals_used.insert(name.to_string());
                    if let Some(var) = dfg.variables.get_mut(name) {
                        var.is_global = true;
                    }
                }
                continue;
            }

            if line.starts_with("nonlocal ") {
                let names = line[9..].split(',');
                for name in names {
                    let name = name.trim();
                    dfg.nonlocals_used.insert(name.to_string());
                    if let Some(var) = dfg.variables.get_mut(name) {
                        var.is_nonlocal = true;
                    }
                }
                continue;
            }

            // Check for return statement
            if line.starts_with("return ") {
                let return_expr = &line[7..];
                let return_vars = self.extract_identifiers(return_expr);
                for var in &return_vars {
                    if !dfg.returns.contains(var) {
                        dfg.returns.push(var.clone());
                    }
                }
            }

            // Extract assignments
            self.analyze_assignments(line, line_idx, func_name, &mut dfg, &mut defined_vars);

            // Extract variable reads
            self.analyze_reads(line, line_idx, func_name, &dfg, &mut defined_vars);
        }

        dfg
    }

    /// Parse function parameters
    fn parse_parameters(&self, args_str: &str, dfg: &mut FunctionDataFlow) {
        for arg in args_str.split(',') {
            let arg = arg.trim();
            if arg.is_empty() {
                continue;
            }

            // Handle *args and **kwargs
            let arg_name = if arg.starts_with("**") {
                &arg[2..]
            } else if arg.starts_with('*') {
                &arg[1..]
            } else {
                arg
            };

            // Remove type hint and default value
            let arg_name = if let Some(colon_pos) = arg_name.find(':') {
                &arg_name[..colon_pos]
            } else if let Some(eq_pos) = arg_name.find('=') {
                &arg_name[..eq_pos]
            } else {
                arg_name
            };

            let arg_name = arg_name.trim();
            if !arg_name.is_empty() && arg_name != "self" && arg_name != "cls" {
                dfg.parameters.push(arg_name.to_string());

                let mut var_info = VariableInfo::new(arg_name, dfg.start_line, &dfg.function_name);
                var_info.is_parameter = true;
                var_info.accesses.push(VariableAccess {
                    name: arg_name.to_string(),
                    action: VarAction::Define,
                    line: dfg.start_line,
                    scope: dfg.function_name.clone(),
                    context: Some("parameter".to_string()),
                });
                dfg.variables.insert(arg_name.to_string(), var_info);
            }
        }
    }

    /// Analyze assignments in a line
    fn analyze_assignments(
        &self,
        line: &str,
        line_num: usize,
        scope: &str,
        dfg: &mut FunctionDataFlow,
        defined_vars: &mut HashSet<String>,
    ) {
        // Simple assignment: name = value
        // Augmented assignment: name += value
        // Multiple assignment: a, b = value

        // Find assignment operators
        let assignment_ops = ["=", "+=", "-=", "*=", "/=", "//=", "%=", "**=", "&=", "|=", "^="];

        for op in &assignment_ops {
            if let Some(eq_pos) = line.find(op) {
                // Make sure it's not ==, !=, <=, >=, :=
                if op == &"=" {
                    if eq_pos > 0 {
                        let prev_char = line.as_bytes().get(eq_pos.saturating_sub(1));
                        if matches!(prev_char, Some(b'!') | Some(b'<') | Some(b'>') | Some(b'=') | Some(b':')) {
                            continue;
                        }
                    }
                    if eq_pos + 1 < line.len() && line.as_bytes().get(eq_pos + 1) == Some(&b'=') {
                        continue;
                    }
                }

                let target_part = &line[..eq_pos];
                let targets = self.extract_assignment_targets(target_part);

                for target in targets {
                    if target.is_empty() {
                        continue;
                    }

                    let action = if defined_vars.contains(&target) || *op != "=" {
                        VarAction::Modify
                    } else {
                        VarAction::Define
                    };

                    defined_vars.insert(target.clone());

                    let var_info = dfg.variables.entry(target.clone()).or_insert_with(|| {
                        VariableInfo::new(&target, line_num, scope)
                    });

                    var_info.accesses.push(VariableAccess {
                        name: target.clone(),
                        action,
                        line: line_num,
                        scope: scope.to_string(),
                        context: Some(line.chars().take(50).collect()),
                    });
                }

                break; // Only process the first assignment operator found
            }
        }

        // Handle for loop variable
        if line.starts_with("for ") {
            if let Some(in_pos) = line.find(" in ") {
                let target_part = &line[4..in_pos];
                let targets = self.extract_assignment_targets(target_part);

                for target in targets {
                    if target.is_empty() {
                        continue;
                    }

                    defined_vars.insert(target.clone());

                    let var_info = dfg.variables.entry(target.clone()).or_insert_with(|| {
                        VariableInfo::new(&target, line_num, scope)
                    });

                    var_info.accesses.push(VariableAccess {
                        name: target.clone(),
                        action: VarAction::Define,
                        line: line_num,
                        scope: scope.to_string(),
                        context: Some("for loop variable".to_string()),
                    });
                }
            }
        }
    }

    /// Extract assignment targets (handles tuple unpacking)
    fn extract_assignment_targets(&self, target_str: &str) -> Vec<String> {
        let mut targets = Vec::new();
        let mut current = String::new();
        let mut bracket_depth = 0;

        for ch in target_str.chars() {
            match ch {
                '(' | '[' | '{' => bracket_depth += 1,
                ')' | ']' | '}' => bracket_depth -= 1,
                ',' if bracket_depth == 0 => {
                    let target = current.trim().to_string();
                    if self.is_valid_identifier(&target) {
                        targets.push(target);
                    }
                    current.clear();
                }
                _ => current.push(ch),
            }
        }

        let target = current.trim().to_string();
        if self.is_valid_identifier(&target) {
            targets.push(target);
        }

        targets
    }

    /// Check if a string is a valid Python identifier
    fn is_valid_identifier(&self, s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        // Skip attribute access and subscripts
        if s.contains('.') || s.contains('[') {
            return false;
        }

        let first = s.chars().next().unwrap();
        if !first.is_alphabetic() && first != '_' {
            return false;
        }

        s.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    /// Analyze variable reads in a line
    fn analyze_reads(
        &self,
        line: &str,
        _line_num: usize,
        _scope: &str,
        dfg: &FunctionDataFlow,
        defined_vars: &HashSet<String>,
    ) {
        let identifiers = self.extract_identifiers(line);

        for ident in identifiers {
            // Skip if this is a function/method call (followed by '(')
            // Skip keywords
            if self.is_keyword(&ident) {
                continue;
            }

            // Skip if not yet defined in this scope
            if !defined_vars.contains(&ident) && !dfg.parameters.contains(&ident) {
                continue;
            }

            // We're not mutating here, so this is just for analysis purposes
            // The actual mutation happens in analyze_assignments
        }
    }

    /// Extract identifiers from a line
    fn extract_identifiers(&self, line: &str) -> Vec<String> {
        let mut identifiers = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut string_char = ' ';

        for ch in line.chars() {
            if (ch == '"' || ch == '\'') && !in_string {
                in_string = true;
                string_char = ch;
                current.clear();
                continue;
            }
            if in_string {
                if ch == string_char {
                    in_string = false;
                }
                continue;
            }

            if ch.is_alphanumeric() || ch == '_' {
                current.push(ch);
            } else {
                if !current.is_empty() && self.is_valid_identifier(&current) && !self.is_keyword(&current) {
                    identifiers.push(current.clone());
                }
                current.clear();
            }
        }

        if !current.is_empty() && self.is_valid_identifier(&current) && !self.is_keyword(&current) {
            identifiers.push(current);
        }

        identifiers
    }

    /// Check if identifier is a Python keyword
    fn is_keyword(&self, s: &str) -> bool {
        let keywords = [
            "False", "None", "True", "and", "as", "assert", "async", "await",
            "break", "class", "continue", "def", "del", "elif", "else", "except",
            "finally", "for", "from", "global", "if", "import", "in", "is",
            "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try",
            "while", "with", "yield",
        ];
        keywords.contains(&s)
    }

    /// Analyze global variables
    fn analyze_globals(&self, lines: &[&str], result: &mut DFGAnalysisResult) {
        for (line_num, raw_line) in lines.iter().enumerate() {
            let line = raw_line.trim();
            let indent = raw_line.len() - raw_line.trim_start().len();

            // Only look at module-level assignments
            if indent > 0 || line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Skip function/class definitions
            if line.starts_with("def ") || line.starts_with("class ") || line.starts_with("async def ") {
                continue;
            }

            // Check for simple assignments
            if let Some(eq_pos) = line.find('=') {
                if eq_pos > 0 {
                    let prev_char = line.as_bytes().get(eq_pos.saturating_sub(1));
                    if matches!(prev_char, Some(b'!') | Some(b'<') | Some(b'>') | Some(b'=') | Some(b':')) {
                        continue;
                    }
                }
                if eq_pos + 1 < line.len() && line.as_bytes().get(eq_pos + 1) == Some(&b'=') {
                    continue;
                }

                let target = line[..eq_pos].trim();
                if self.is_valid_identifier(target) {
                    let mut var_info = VariableInfo::new(target, line_num + 1, "__module__");
                    var_info.is_global = true;
                    var_info.accesses.push(VariableAccess {
                        name: target.to_string(),
                        action: VarAction::Define,
                        line: line_num + 1,
                        scope: "__module__".to_string(),
                        context: Some(line.chars().take(50).collect()),
                    });
                    result.global_variables.insert(target.to_string(), var_info);
                }
            }
        }
    }

    /// Convert analysis result to LLM-friendly string
    pub fn to_llm_string(&self, result: &DFGAnalysisResult) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "# DFG Analysis: {} ({} functions)",
            result.file_path,
            result.functions.len()
        ));

        // Global variables
        if !result.global_variables.is_empty() {
            lines.push(String::new());
            lines.push("## Global Variables".to_string());
            for (name, var) in result.global_variables.iter().take(10) {
                lines.push(format!("  {} L{}", name, var.defining_line));
            }
        }

        // Functions
        for (func_name, dfg) in &result.functions {
            lines.push(String::new());
            lines.push(format!("## Function: {}", func_name));

            // Parameters
            if !dfg.parameters.is_empty() {
                lines.push(format!("  Parameters: {}", dfg.parameters.join(", ")));
            }

            // Returns
            if !dfg.returns.is_empty() {
                lines.push(format!("  Returns: {}", dfg.returns.join(", ")));
            }

            // Key variables
            let var_summaries: Vec<String> = dfg
                .variables
                .iter()
                .filter(|(_, v)| v.accesses.len() > 1 || v.is_parameter)
                .map(|(name, v)| {
                    let uses = v.read_count() + v.modify_count();
                    format!("{}(L{},uses:{})", name, v.defining_line, uses)
                })
                .take(10)
                .collect();

            if !var_summaries.is_empty() {
                lines.push(format!("  Variables: {}", var_summaries.join(" ")));
            }

            // Globals/nonlocals used
            if !dfg.globals_used.is_empty() {
                lines.push(format!(
                    "  Globals: {}",
                    dfg.globals_used.iter().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
        }

        lines.join("\n")
    }
}

impl Default for DFGAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function_dfg() {
        let source = r#"
def add(a, b):
    result = a + b
    return result
"#;
        let mut analyzer = DFGAnalyzer::new();
        let result = analyzer.analyze(source, "test.py");

        assert_eq!(result.functions.len(), 1);
        let dfg = result.functions.get("add").unwrap();
        assert_eq!(dfg.parameters, vec!["a", "b"]);
        assert!(dfg.variables.contains_key("result"));
    }

    #[test]
    fn test_global_variables() {
        let source = r#"
CONFIG = {}
DEBUG = True

def setup():
    global CONFIG
    CONFIG = {"debug": DEBUG}
"#;
        let mut analyzer = DFGAnalyzer::new();
        let result = analyzer.analyze(source, "test.py");

        assert!(result.global_variables.contains_key("CONFIG"));
        assert!(result.global_variables.contains_key("DEBUG"));
    }
}
