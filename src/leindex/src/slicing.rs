//! Slicing Analyzer (Layer 5)
//!
//! Program slicing: finds all statements that affect (or are affected by) a given point.
//! Combines CFG (control dependencies) and DFG (data dependencies) into a PDG.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

// Imports filtered to remove unused analysts

/// Direction of program slice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SliceDirection {
    Backward,
    Forward,
    Both,
}

/// Type of program dependence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependenceType {
    Control,
    Data,
}

/// A dependence edge in the PDG
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DependenceEdge {
    pub from_line: usize,
    pub to_line: usize,
    pub dep_type: DependenceType,
    pub variable: Option<String>, // For data dependencies
}

/// Program Dependence Graph (PDG)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramDependenceGraph {
    pub function_name: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub edges: HashSet<DependenceEdge>,
    pub lines_with_statements: HashSet<usize>,
}

impl ProgramDependenceGraph {
    pub fn new(function_name: &str, file_path: &str, start_line: usize, end_line: usize) -> Self {
        Self {
            function_name: function_name.to_string(),
            file_path: file_path.to_string(),
            start_line,
            end_line,
            edges: HashSet::new(),
            lines_with_statements: HashSet::new(),
        }
    }

    /// Get all lines that influence the given line (predecessors)
    pub fn get_predecessors(&self, line: usize) -> HashSet<usize> {
        self.edges
            .iter()
            .filter(|e| e.to_line == line)
            .map(|e| e.from_line)
            .collect()
    }

    /// Get all lines influenced by the given line (successors)
    pub fn get_successors(&self, line: usize) -> HashSet<usize> {
        self.edges
            .iter()
            .filter(|e| e.from_line == line)
            .map(|e| e.to_line)
            .collect()
    }

    /// Add a dependence edge
    pub fn add_edge(
        &mut self,
        from_line: usize,
        to_line: usize,
        dep_type: DependenceType,
        variable: Option<String>,
    ) {
        self.edges.insert(DependenceEdge {
            from_line,
            to_line,
            dep_type,
            variable,
        });
    }
}

/// Result of a program slice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceResult {
    pub function_name: String,
    pub target_line: usize,
    pub direction: SliceDirection,
    pub relevant_lines: HashSet<usize>,
    pub relevant_variables: HashSet<String>,
    pub dependencies: Vec<(usize, String)>, // (line, variables involved)
}

impl SliceResult {
    pub fn new(function_name: &str, target_line: usize, direction: SliceDirection) -> Self {
        Self {
            function_name: function_name.to_string(),
            target_line,
            direction,
            relevant_lines: HashSet::new(),
            relevant_variables: HashSet::new(),
            dependencies: Vec::new(),
        }
    }
}

/// PDG analysis result for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PDGAnalysisResult {
    pub file_path: String,
    pub functions: HashMap<String, ProgramDependenceGraph>,
}

/// Slicing Analyzer
pub struct SlicingAnalyzer;

impl SlicingAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a file and build PDGs for all functions
    pub fn analyze(&self, source: &str, file_path: &str) -> PDGAnalysisResult {
        let mut result = PDGAnalysisResult {
            file_path: file_path.to_string(),
            functions: HashMap::new(),
        };

        let lines: Vec<&str> = source.lines().collect();

        // Find all function definitions
        let mut function_ranges: Vec<(String, usize, usize)> = Vec::new();

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

                    function_ranges.push((func_name, line_num + 1, end_line));
                }
            }
        }

        // Build PDG for each function
        for (func_name, start_line, end_line) in function_ranges {
            let pdg = self.build_pdg(&lines, &func_name, file_path, start_line, end_line);
            result.functions.insert(func_name, pdg);
        }

        result
    }

    /// Build a Program Dependence Graph for a function
    pub fn build_pdg(
        &self,
        lines: &[&str],
        function_name: &str,
        file_path: &str,
        start_line: usize,
        end_line: usize,
    ) -> ProgramDependenceGraph {
        let mut pdg = ProgramDependenceGraph::new(function_name, file_path, start_line, end_line);

        // Track which lines have actual statements
        for line_idx in start_line..=end_line.min(lines.len()) {
            if line_idx == 0 || line_idx > lines.len() {
                continue;
            }
            let line = lines[line_idx - 1].trim();
            if !line.is_empty() && !line.starts_with('#') {
                pdg.lines_with_statements.insert(line_idx);
            }
        }

        // Add control dependencies
        self.add_control_dependencies(&mut pdg, lines, start_line, end_line);

        // Add data dependencies
        self.add_data_dependencies(&mut pdg, lines, start_line, end_line);

        pdg
    }

    /// Add control flow dependencies to PDG
    fn add_control_dependencies(
        &self,
        pdg: &mut ProgramDependenceGraph,
        lines: &[&str],
        start_line: usize,
        end_line: usize,
    ) {
        let mut control_stack: Vec<usize> = Vec::new(); // Stack of control statement lines
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

            // Pop control statements that we've exited
            while !indent_stack.is_empty() && indent <= *indent_stack.last().unwrap() {
                indent_stack.pop();
                control_stack.pop();
            }

            // If there's a controlling statement, add dependency
            if let Some(&control_line) = control_stack.last() {
                pdg.add_edge(control_line, line_idx, DependenceType::Control, None);
            }

            // Check if this is a control statement
            if line.starts_with("if ")
                || line.starts_with("elif ")
                || line.starts_with("else:")
                || line.starts_with("for ")
                || line.starts_with("while ")
                || line.starts_with("try:")
                || line.starts_with("except")
                || line.starts_with("finally:")
                || line.starts_with("with ")
                || line.starts_with("match ")
                || line.starts_with("case ")
            {
                control_stack.push(line_idx);
                indent_stack.push(indent);
            }
        }
    }

    /// Add data flow dependencies to PDG
    fn add_data_dependencies(
        &self,
        pdg: &mut ProgramDependenceGraph,
        lines: &[&str],
        start_line: usize,
        end_line: usize,
    ) {
        // Track variable definitions: variable -> list of (line, action)
        let mut var_defs: HashMap<String, Vec<usize>> = HashMap::new();

        for line_idx in start_line..=end_line.min(lines.len()) {
            if line_idx == 0 || line_idx > lines.len() {
                continue;
            }
            let raw_line = lines[line_idx - 1];
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Extract variables used (reads)
            let reads = self.extract_reads(line);

            // For each read, add a data dependency from the last definition
            for var in &reads {
                if let Some(def_lines) = var_defs.get(var) {
                    if let Some(&last_def) = def_lines.last() {
                        if last_def != line_idx {
                            pdg.add_edge(
                                last_def,
                                line_idx,
                                DependenceType::Data,
                                Some(var.clone()),
                            );
                        }
                    }
                }
            }

            // Extract variables defined
            let defs = self.extract_definitions(line);

            // Update definition points
            for var in defs {
                var_defs.entry(var).or_default().push(line_idx);
            }
        }
    }

    /// Extract variable reads from a line
    fn extract_reads(&self, line: &str) -> Vec<String> {
        // This is a simplified extraction - just get identifiers on the RHS of assignments
        // or anywhere in expressions
        let mut reads = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut string_char = ' ';

        // Skip the LHS of assignments
        let rhs = if let Some(eq_pos) = self.find_assignment_pos(line) {
            &line[eq_pos + 1..]
        } else {
            line
        };

        for ch in rhs.chars() {
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
                if !current.is_empty() && self.is_valid_identifier(&current) {
                    reads.push(current.clone());
                }
                current.clear();
            }
        }

        if !current.is_empty() && self.is_valid_identifier(&current) {
            reads.push(current);
        }

        reads
    }

    /// Extract variable definitions from a line
    fn extract_definitions(&self, line: &str) -> Vec<String> {
        let mut defs = Vec::new();

        // Check for assignments
        if let Some(eq_pos) = self.find_assignment_pos(line) {
            let lhs = &line[..eq_pos];
            let targets = self.extract_targets(lhs);
            defs.extend(targets);
        }

        // Check for for loop variables
        if line.starts_with("for ") {
            if let Some(in_pos) = line.find(" in ") {
                let target_part = &line[4..in_pos];
                defs.extend(self.extract_targets(target_part));
            }
        }

        // Check for except clause with 'as'
        if line.starts_with("except ") {
            if let Some(as_pos) = line.find(" as ") {
                let var_part = &line[as_pos + 4..];
                let var = var_part.trim().trim_end_matches(':');
                if self.is_valid_identifier(var) {
                    defs.push(var.to_string());
                }
            }
        }

        // Check for with statement
        if line.starts_with("with ") {
            if let Some(as_pos) = line.find(" as ") {
                let var_part = &line[as_pos + 4..];
                let var = var_part.trim().trim_end_matches(':');
                if self.is_valid_identifier(var) {
                    defs.push(var.to_string());
                }
            }
        }

        defs
    }

    /// Find position of assignment operator in line
    fn find_assignment_pos(&self, line: &str) -> Option<usize> {
        let assignment_ops = ["=", "+=", "-=", "*=", "/=", "//=", "%=", "**="];

        for op in &assignment_ops {
            if let Some(pos) = line.find(op) {
                // Verify it's not ==, !=, <=, >=, :=
                if *op == "=" {
                    if pos > 0 {
                        let prev = line.as_bytes().get(pos.saturating_sub(1));
                        if matches!(
                            prev,
                            Some(b'!') | Some(b'<') | Some(b'>') | Some(b'=') | Some(b':')
                        ) {
                            continue;
                        }
                    }
                    if pos + 1 < line.len() && line.as_bytes().get(pos + 1) == Some(&b'=') {
                        continue;
                    }
                }
                return Some(pos);
            }
        }

        None
    }

    /// Extract assignment targets
    fn extract_targets(&self, target_str: &str) -> Vec<String> {
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

    /// Check if identifier is valid
    fn is_valid_identifier(&self, s: &str) -> bool {
        if s.is_empty() || s.contains('.') || s.contains('[') {
            return false;
        }

        let keywords = [
            "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
            "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
            "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise",
            "return", "try", "while", "with", "yield", "print", "len", "range", "str", "int",
        ];

        if keywords.contains(&s) {
            return false;
        }

        let first = s.chars().next().unwrap();
        if !first.is_alphabetic() && first != '_' {
            return false;
        }

        s.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    /// Perform backward slice from a target line
    pub fn slice_backward(
        &self,
        source: &str,
        function_name: &str,
        target_line: usize,
        file_path: &str,
    ) -> SliceResult {
        let lines: Vec<&str> = source.lines().collect();

        // Find the function
        let mut func_start = 0;
        let mut func_end = 0;

        for (line_num, raw_line) in lines.iter().enumerate() {
            let line = raw_line.trim();
            let indent = raw_line.len() - raw_line.trim_start().len();

            if line.starts_with("def ") || line.starts_with("async def ") {
                let is_async = line.starts_with("async def ");
                let rest = if is_async { &line[10..] } else { &line[4..] };
                if let Some(paren_pos) = rest.find('(') {
                    let name = rest[..paren_pos].trim();
                    if name == function_name {
                        func_start = line_num + 1;

                        // Find end
                        for (idx, next_line) in lines.iter().enumerate().skip(line_num + 1) {
                            let next_trimmed = next_line.trim();
                            let next_indent = next_line.len() - next_line.trim_start().len();

                            if next_trimmed.is_empty() || next_trimmed.starts_with('#') {
                                continue;
                            }

                            if next_indent <= indent {
                                break;
                            }
                            func_end = idx + 1;
                        }
                        break;
                    }
                }
            }
        }

        let mut result = SliceResult::new(function_name, target_line, SliceDirection::Backward);

        if func_start == 0 {
            return result; // Function not found
        }

        // Build PDG
        let pdg = self.build_pdg(&lines, function_name, file_path, func_start, func_end);

        // BFS backward from target line
        let mut visited: HashSet<usize> = HashSet::new();
        let mut queue: VecDeque<usize> = VecDeque::new();

        // Include target line
        result.relevant_lines.insert(target_line);
        visited.insert(target_line);

        // Add predecessors of target
        for pred in pdg.get_predecessors(target_line) {
            if !visited.contains(&pred) {
                queue.push_back(pred);
            }
        }

        // BFS
        while let Some(line) = queue.pop_front() {
            if visited.contains(&line) {
                continue;
            }

            visited.insert(line);
            result.relevant_lines.insert(line);

            // Add predecessors
            for pred in pdg.get_predecessors(line) {
                if !visited.contains(&pred) {
                    queue.push_back(pred);
                }
            }
        }

        // Extract relevant variables from data edges
        for edge in &pdg.edges {
            #[allow(clippy::collapsible_if)]
            if edge.dep_type == DependenceType::Data {
                if result.relevant_lines.contains(&edge.from_line)
                    || result.relevant_lines.contains(&edge.to_line)
                {
                    if let Some(ref var) = edge.variable {
                        result.relevant_variables.insert(var.clone());
                    }
                }
            }
        }

        // Build dependency list
        let mut sorted_lines: Vec<usize> = result.relevant_lines.iter().cloned().collect();
        sorted_lines.sort();

        for line in sorted_lines {
            let vars_at_line: Vec<String> = pdg
                .edges
                .iter()
                .filter(|e| e.from_line == line || e.to_line == line)
                .filter_map(|e| e.variable.clone())
                .collect();

            if !vars_at_line.is_empty() {
                let unique_vars: HashSet<String> = vars_at_line.into_iter().collect();
                result
                    .dependencies
                    .push((line, unique_vars.into_iter().collect::<Vec<_>>().join(", ")));
            }
        }

        result
    }

    /// Perform forward slice from a target line
    pub fn slice_forward(
        &self,
        source: &str,
        function_name: &str,
        target_line: usize,
        file_path: &str,
    ) -> SliceResult {
        let lines: Vec<&str> = source.lines().collect();

        // Find the function (same as backward slice)
        let mut func_start = 0;
        let mut func_end = 0;

        for (line_num, raw_line) in lines.iter().enumerate() {
            let line = raw_line.trim();
            let indent = raw_line.len() - raw_line.trim_start().len();

            if line.starts_with("def ") || line.starts_with("async def ") {
                let is_async = line.starts_with("async def ");
                let rest = if is_async { &line[10..] } else { &line[4..] };
                if let Some(paren_pos) = rest.find('(') {
                    let name = rest[..paren_pos].trim();
                    if name == function_name {
                        func_start = line_num + 1;

                        for (idx, next_line) in lines.iter().enumerate().skip(line_num + 1) {
                            let next_trimmed = next_line.trim();
                            let next_indent = next_line.len() - next_line.trim_start().len();

                            if next_trimmed.is_empty() || next_trimmed.starts_with('#') {
                                continue;
                            }

                            if next_indent <= indent {
                                break;
                            }
                            func_end = idx + 1;
                        }
                        break;
                    }
                }
            }
        }

        let mut result = SliceResult::new(function_name, target_line, SliceDirection::Forward);

        if func_start == 0 {
            return result;
        }

        let pdg = self.build_pdg(&lines, function_name, file_path, func_start, func_end);

        // BFS forward from target line
        let mut visited: HashSet<usize> = HashSet::new();
        let mut queue: VecDeque<usize> = VecDeque::new();

        result.relevant_lines.insert(target_line);
        visited.insert(target_line);

        for succ in pdg.get_successors(target_line) {
            if !visited.contains(&succ) {
                queue.push_back(succ);
            }
        }

        while let Some(line) = queue.pop_front() {
            if visited.contains(&line) {
                continue;
            }

            visited.insert(line);
            result.relevant_lines.insert(line);

            for succ in pdg.get_successors(line) {
                if !visited.contains(&succ) {
                    queue.push_back(succ);
                }
            }
        }

        // Extract relevant variables
        for edge in &pdg.edges {
            if edge.dep_type == DependenceType::Data {
                if result.relevant_lines.contains(&edge.from_line)
                    || result.relevant_lines.contains(&edge.to_line)
                {
                    if let Some(ref var) = edge.variable {
                        result.relevant_variables.insert(var.clone());
                    }
                }
            }
        }

        result
    }

    /// Convert slice result to LLM-friendly string
    pub fn to_llm_string(&self, result: &SliceResult) -> String {
        let mut lines = Vec::new();

        let direction_str = match result.direction {
            SliceDirection::Backward => "backward",
            SliceDirection::Forward => "forward",
            SliceDirection::Both => "both",
        };

        lines.push(format!(
            "# {} Slice: {} @ L{}",
            direction_str, result.function_name, result.target_line
        ));

        lines.push(format!(
            "Relevant lines: {} lines",
            result.relevant_lines.len()
        ));

        if !result.relevant_variables.is_empty() {
            lines.push(format!(
                "Variables involved: {}",
                result
                    .relevant_variables
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Show relevant lines in order
        if !result.relevant_lines.is_empty() {
            lines.push(String::new());
            lines.push("## Relevant Lines".to_string());

            let mut sorted: Vec<usize> = result.relevant_lines.iter().cloned().collect();
            sorted.sort();

            for line in sorted.iter().take(20) {
                let marker = if *line == result.target_line {
                    " <-"
                } else {
                    ""
                };
                lines.push(format!("  L{}{}", line, marker));
            }

            if sorted.len() > 20 {
                lines.push(format!("  ... and {} more lines", sorted.len() - 20));
            }
        }

        // Show dependencies
        if !result.dependencies.is_empty() {
            lines.push(String::new());
            lines.push("## Data Dependencies".to_string());
            for (line, vars) in result.dependencies.iter().take(10) {
                lines.push(format!("  L{}: {}", line, vars));
            }
        }

        lines.join("\n")
    }
}

impl Default for SlicingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backward_slice() {
        let source = r#"
def compute(x, y):
    a = x + 1
    b = y + 2
    c = a * b
    d = c + 10
    return d
"#;
        let analyzer = SlicingAnalyzer::new();
        let result = analyzer.slice_backward(source, "compute", 6, "test.py");

        // Line 6 (d = c + 10) should depend on c (line 5)
        // c depends on a (line 3) and b (line 4)
        // a depends on x (parameter), b depends on y (parameter)
        assert!(result.relevant_lines.contains(&6)); // target
        assert!(result.relevant_lines.contains(&5)); // c = a * b
        assert!(result.relevant_lines.contains(&3)); // a = x + 1
        assert!(result.relevant_lines.contains(&4)); // b = y + 2
    }

    #[test]
    fn test_forward_slice() {
        let source = r#"
def process(data):
    x = data[0]
    y = data[1]
    result = x + y
    print(result)
    z = 42
    return result
"#;
        let analyzer = SlicingAnalyzer::new();
        let result = analyzer.slice_forward(source, "process", 3, "test.py");

        // x defined at line 3 should influence:
        // - line 5 (result = x + y)
        // - line 6 (print(result))
        // - line 8 (return result)
        assert!(result.relevant_lines.contains(&3)); // x = data[0]
    }

    #[test]
    fn test_pdg_construction() {
        let source = r#"
def example(n):
    total = 0
    for i in range(n):
        if i % 2 == 0:
            total += i
    return total
"#;
        let lines: Vec<&str> = source.lines().collect();
        let analyzer = SlicingAnalyzer::new();
        let pdg = analyzer.build_pdg(&lines, "example", "test.py", 2, 7);

        // Should have both control and data dependencies
        let control_edges: Vec<_> = pdg
            .edges
            .iter()
            .filter(|e| e.dep_type == DependenceType::Control)
            .collect();
        let data_edges: Vec<_> = pdg
            .edges
            .iter()
            .filter(|e| e.dep_type == DependenceType::Data)
            .collect();

        assert!(!control_edges.is_empty());
        assert!(!data_edges.is_empty());
    }
}
