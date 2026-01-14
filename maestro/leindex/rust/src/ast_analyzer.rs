//! AST Analyzer (Layer 1)
//!
//! Extracts code structure: function signatures, imports, classes
//! Uses line-by-line parsing with indentation tracking to properly
//! associate methods with classes.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Information about an import statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub module: String,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub line: usize,
}

/// Information about a function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line: usize,
    pub args: String,
    pub returns: Option<String>,
    pub is_async: bool,
    pub is_method: bool,
    pub class_name: Option<String>,
    pub decorators: Vec<String>,
    pub docstring: Option<String>,
    pub calls: HashSet<String>,
}

/// Information about a class definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub line: usize,
    pub bases: Vec<String>,
    pub decorators: Vec<String>,
    pub docstring: Option<String>,
    pub methods: Vec<FunctionInfo>,
}

/// Complete AST analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASTAnalysis {
    pub file_path: String,
    pub imports: Vec<ImportInfo>,
    pub classes: Vec<ClassInfo>,
    pub functions: Vec<FunctionInfo>,
    pub globals: Vec<String>,
    pub line_count: usize,
    pub module_docstring: Option<String>,
}

/// AST Analyzer for extracting Python code structure
pub struct ASTAnalyzer {
    imports: Vec<ImportInfo>,
    classes: Vec<ClassInfo>,
    functions: Vec<FunctionInfo>,
    globals: Vec<String>,
    file_path: String,
    line_count: usize,
    module_docstring: Option<String>,
}

impl ASTAnalyzer {
    pub fn new() -> Self {
        Self {
            imports: Vec::new(),
            classes: Vec::new(),
            functions: Vec::new(),
            globals: Vec::new(),
            file_path: String::new(),
            line_count: 0,
            module_docstring: None,
        }
    }

    /// Analyze Python source code
    pub fn analyze(&mut self, source: &str, file_path: &str) -> ASTAnalysis {
        self.file_path = file_path.to_string();
        self.imports.clear();
        self.classes.clear();
        self.functions.clear();
        self.globals.clear();
        self.module_docstring = None;

        let lines: Vec<&str> = source.lines().collect();
        self.line_count = lines.len();

        // Track class context using indentation
        let mut current_class: Option<(String, usize)> = None; // (class_name, class_indent)
        let mut pending_decorators: Vec<String> = Vec::new();
        let mut in_multiline_string = false;
        let mut multiline_quote = "";

        for (line_num, raw_line) in lines.iter().enumerate() {
            let line_number = line_num + 1; // 1-indexed

            // Handle multiline strings
            if in_multiline_string {
                if raw_line.contains(multiline_quote) {
                    in_multiline_string = false;
                }
                continue;
            }

            // Check for multiline string start
            if raw_line.contains("\"\"\"") || raw_line.contains("'''") {
                let quote = if raw_line.contains("\"\"\"") { "\"\"\"" } else { "'''" };
                let count = raw_line.matches(quote).count();
                if count == 1 {
                    in_multiline_string = true;
                    multiline_quote = quote;
                    // Check if this is a module docstring (first non-empty, non-comment line)
                    if self.module_docstring.is_none()
                        && self.imports.is_empty()
                        && self.classes.is_empty()
                        && self.functions.is_empty()
                    {
                        // Extract docstring content
                        let start_pos = raw_line.find(quote).unwrap_or(0);
                        let content = &raw_line[start_pos + 3..];
                        if !content.is_empty() {
                            self.module_docstring = Some(content.chars().take(100).collect());
                        }
                    }
                }
                continue;
            }

            let line = raw_line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Calculate indentation
            let indent = raw_line.len() - raw_line.trim_start().len();

            // Check if we've exited a class (dedented beyond class level)
            if let Some((_, class_indent)) = &current_class {
                if indent <= *class_indent && !line.starts_with('@') {
                    // We've exited the class
                    current_class = None;
                }
            }

            // Track decorators
            if line.starts_with('@') {
                let decorator = line[1..].split('(').next().unwrap_or(line);
                pending_decorators.push(decorator.to_string());
                continue;
            }

            // Extract imports
            if line.starts_with("from ") || line.starts_with("import ") {
                if let Some(imp) = self.parse_import(line, line_number) {
                    self.imports.push(imp);
                }
                pending_decorators.clear();
                continue;
            }

            // Extract class definitions
            if line.starts_with("class ") {
                if let Some(mut cls) = self.parse_class(line, line_number) {
                    cls.decorators = pending_decorators.clone();
                    current_class = Some((cls.name.clone(), indent));
                    self.classes.push(cls);
                }
                pending_decorators.clear();
                continue;
            }

            // Extract function definitions
            if line.starts_with("def ") || line.starts_with("async def ") {
                if let Some(mut func) = self.parse_function(line, line_number) {
                    func.decorators = pending_decorators.clone();

                    // Check if this is a method (inside a class)
                    if let Some((class_name, _)) = &current_class {
                        func.is_method = true;
                        func.class_name = Some(class_name.clone());

                        // Add to the class's methods list
                        if let Some(cls) = self.classes.iter_mut().find(|c| c.name == *class_name)
                        {
                            cls.methods.push(func.clone());
                        }
                    } else {
                        self.functions.push(func);
                    }
                }
                pending_decorators.clear();
                continue;
            }

            // Track global assignments (only at module level)
            if current_class.is_none() && indent == 0 {
                if let Some(var_name) = self.parse_global_assignment(line) {
                    self.globals.push(var_name);
                }
            }

            pending_decorators.clear();
        }

        // Extract function calls from the source
        self.extract_function_calls(source);

        ASTAnalysis {
            file_path: self.file_path.clone(),
            imports: self.imports.clone(),
            classes: self.classes.clone(),
            functions: self.functions.clone(),
            globals: self.globals.clone(),
            line_count: self.line_count,
            module_docstring: self.module_docstring.clone(),
        }
    }

    fn parse_import(&self, line: &str, line_num: usize) -> Option<ImportInfo> {
        if line.starts_with("from ") {
            // from module import name [as alias]
            let rest = &line[5..];
            if let Some(import_pos) = rest.find(" import ") {
                let module = rest[..import_pos].trim().to_string();
                let imports_part = rest[import_pos + 8..].trim();

                // Handle multiple imports: from x import a, b, c
                // We'll just take the first one for simplicity, or handle comma-separated
                for import_item in imports_part.split(',') {
                    let import_item = import_item.trim();
                    if import_item.is_empty() || import_item == "(" {
                        continue;
                    }

                    // Handle 'as' alias
                    let (name, alias) = if let Some(as_pos) = import_item.find(" as ") {
                        (
                            import_item[..as_pos].trim().to_string(),
                            Some(import_item[as_pos + 4..].trim().to_string()),
                        )
                    } else {
                        (import_item.trim_end_matches(',').to_string(), None)
                    };

                    return Some(ImportInfo {
                        module: module.clone(),
                        name: Some(name),
                        alias,
                        line: line_num,
                    });
                }
            }
        } else if line.starts_with("import ") {
            // import module [as alias]
            let rest = &line[7..];
            let (module, alias) = if let Some(as_pos) = rest.find(" as ") {
                (
                    rest[..as_pos].trim().to_string(),
                    Some(rest[as_pos + 4..].trim().to_string()),
                )
            } else {
                (rest.trim().to_string(), None)
            };

            return Some(ImportInfo {
                module,
                name: None,
                alias,
                line: line_num,
            });
        }

        None
    }

    fn parse_function(&self, line: &str, line_num: usize) -> Option<FunctionInfo> {
        let is_async = line.starts_with("async def ");
        let rest = if is_async { &line[10..] } else { &line[4..] };

        // Find function name
        let name_end = rest.find('(')?;
        let name = rest[..name_end].trim().to_string();

        // Extract arguments
        let args_start = name_end;
        let args_end = rest.rfind(')')?;
        let args_raw = &rest[args_start + 1..args_end];
        let args = self.condense_args(args_raw);

        // Extract return type
        let returns = if let Some(arrow_pos) = rest.rfind("->") {
            let ret_part = rest[arrow_pos + 2..].trim();
            // Remove trailing colon
            let ret = ret_part.trim_end_matches(':').trim();
            if ret.is_empty() {
                None
            } else {
                Some(self.condense_type(ret))
            }
        } else {
            None
        };

        Some(FunctionInfo {
            name,
            line: line_num,
            args,
            returns,
            is_async,
            is_method: false,
            class_name: None,
            decorators: Vec::new(),
            docstring: None,
            calls: HashSet::new(),
        })
    }

    fn parse_class(&self, line: &str, line_num: usize) -> Option<ClassInfo> {
        let rest = &line[6..]; // Skip "class "

        // Find class name
        let name_end = rest
            .find('(')
            .or_else(|| rest.find(':'))
            .unwrap_or(rest.len());
        let name = rest[..name_end].trim().to_string();

        // Extract base classes
        let bases = if let Some(paren_pos) = rest.find('(') {
            let close_pos = rest.rfind(')')?;
            let bases_str = &rest[paren_pos + 1..close_pos];
            self.parse_bases(bases_str)
        } else {
            Vec::new()
        };

        Some(ClassInfo {
            name,
            line: line_num,
            bases,
            decorators: Vec::new(),
            docstring: None,
            methods: Vec::new(),
        })
    }

    fn parse_bases(&self, bases_str: &str) -> Vec<String> {
        let mut bases = Vec::new();
        let mut current = String::new();
        let mut bracket_depth = 0;

        for ch in bases_str.chars() {
            match ch {
                '[' | '(' => {
                    bracket_depth += 1;
                    current.push(ch);
                }
                ']' | ')' => {
                    bracket_depth -= 1;
                    current.push(ch);
                }
                ',' if bracket_depth == 0 => {
                    let base = current.trim().to_string();
                    if !base.is_empty() {
                        bases.push(self.condense_type(&base));
                    }
                    current.clear();
                }
                _ => current.push(ch),
            }
        }

        let base = current.trim().to_string();
        if !base.is_empty() {
            bases.push(self.condense_type(&base));
        }

        bases
    }

    fn parse_global_assignment(&self, line: &str) -> Option<String> {
        // Look for simple assignments: NAME = ...
        if let Some(eq_pos) = line.find('=') {
            // Make sure it's not ==, !=, <=, >=, etc.
            if eq_pos > 0 {
                let before_eq = &line[..eq_pos];
                let char_before = before_eq.chars().last().unwrap_or(' ');
                if char_before == '!' || char_before == '<' || char_before == '>' || char_before == '=' {
                    return None;
                }
            }
            if eq_pos + 1 < line.len() && line.as_bytes().get(eq_pos + 1) == Some(&b'=') {
                return None;
            }

            let target = line[..eq_pos].trim();
            // Check if it's a simple name (not attribute access, not subscript)
            if target.chars().all(|c| c.is_alphanumeric() || c == '_')
                && target.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_')
            {
                return Some(target.to_string());
            }
        }
        None
    }

    fn condense_args(&self, args: &str) -> String {
        let parts: Vec<&str> = args.split(',').collect();
        let condensed: Vec<String> = parts
            .iter()
            .take(5)
            .map(|arg| {
                let arg = arg.trim();
                if arg.is_empty() {
                    return String::new();
                }

                // Handle *args, **kwargs
                if arg.starts_with("**") || arg.starts_with('*') {
                    return arg.to_string();
                }

                // Strip default values
                let arg = if let Some(eq_pos) = arg.find('=') {
                    arg[..eq_pos].trim()
                } else {
                    arg
                };

                // Condense type hints
                if let Some(colon_pos) = arg.find(':') {
                    let name = &arg[..colon_pos];
                    let type_hint = &arg[colon_pos + 1..];
                    let condensed_type = self.condense_type(type_hint.trim());
                    if condensed_type.len() > 15 {
                        format!("{}", name.trim())
                    } else {
                        format!("{}:{}", name.trim(), condensed_type)
                    }
                } else {
                    arg.to_string()
                }
            })
            .filter(|s| !s.is_empty())
            .collect();

        let mut result = condensed.join(", ");
        if parts.len() > 5 {
            result.push_str(", ...");
        }
        result
    }

    fn condense_type(&self, type_str: &str) -> String {
        let mut result = type_str.to_string();

        // Common abbreviations
        let abbreviations = [
            ("Optional", "Opt"),
            ("Callable", "Fn"),
            ("Awaitable", "Aw"),
            ("AsyncIterator", "AIt"),
            ("Iterator", "It"),
            ("Generator", "Gen"),
            ("Sequence", "Seq"),
            ("Mapping", "Map"),
            ("MutableMapping", "MMap"),
            ("MutableSequence", "MSeq"),
            ("Coroutine", "Coro"),
        ];

        for (full, abbr) in &abbreviations {
            result = result.replace(full, abbr);
        }

        // Truncate if still too long
        if result.len() > 25 {
            result = format!("{}...", &result[..22]);
        }

        result
    }

    fn extract_function_calls(&mut self, source: &str) {
        // Simple regex-free extraction of function calls
        // Look for patterns like: name( or name.method(
        let mut chars = source.chars().peekable();
        let mut current_name = String::new();
        let mut in_string = false;
        let mut string_char = ' ';

        while let Some(ch) = chars.next() {
            // Handle strings
            if (ch == '"' || ch == '\'') && !in_string {
                in_string = true;
                string_char = ch;
                current_name.clear();
                continue;
            }
            if in_string && ch == string_char {
                in_string = false;
                continue;
            }
            if in_string {
                continue;
            }

            // Build identifier
            if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                current_name.push(ch);
            } else if ch == '(' && !current_name.is_empty() {
                // This is a function call
                let call_name = current_name.clone();
                current_name.clear();

                // Find which function this call belongs to
                // For simplicity, we'll add it to all functions
                // A proper implementation would track scope
                for func in &mut self.functions {
                    func.calls.insert(call_name.clone());
                }
                for cls in &mut self.classes {
                    for method in &mut cls.methods {
                        method.calls.insert(call_name.clone());
                    }
                }
            } else {
                current_name.clear();
            }
        }
    }

    /// Convert to LLM-friendly string (balanced mode)
    pub fn to_llm_string(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("## {} ({} lines)", self.file_path, self.line_count));

        // Module docstring
        if let Some(ref doc) = self.module_docstring {
            lines.push(format!("\"\"\"{}\"\"\"", doc));
        }

        // Imports (condensed)
        if !self.imports.is_empty() {
            lines.push(String::new());
            lines.push("# Imports".to_string());
            for imp in self.imports.iter().take(15) {
                if let Some(ref name) = imp.name {
                    let alias_str = imp
                        .alias
                        .as_ref()
                        .map(|a| format!(" as {}", a))
                        .unwrap_or_default();
                    lines.push(format!("from {} import {}{}", imp.module, name, alias_str));
                } else {
                    let alias_str = imp
                        .alias
                        .as_ref()
                        .map(|a| format!(" as {}", a))
                        .unwrap_or_default();
                    lines.push(format!("import {}{}", imp.module, alias_str));
                }
            }
            if self.imports.len() > 15 {
                lines.push(format!("# ... and {} more imports", self.imports.len() - 15));
            }
        }

        // Classes
        if !self.classes.is_empty() {
            lines.push(String::new());
            lines.push("# Classes".to_string());
            for cls in &self.classes {
                let bases = if !cls.bases.is_empty() {
                    format!("({})", cls.bases.join(", "))
                } else {
                    String::new()
                };

                let decorators = if !cls.decorators.is_empty() {
                    format!(
                        " @{}",
                        cls.decorators
                            .iter()
                            .take(3)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(",@")
                    )
                } else {
                    String::new()
                };

                lines.push(format!(
                    "class {}{}: L{}{}",
                    cls.name, bases, cls.line, decorators
                ));

                // Methods
                for method in &cls.methods {
                    let async_prefix = if method.is_async { "async " } else { "" };
                    let ret = method
                        .returns
                        .as_ref()
                        .map(|r| format!(" -> {}", r))
                        .unwrap_or_default();
                    lines.push(format!(
                        "  {}def {}({}){} L{}",
                        async_prefix, method.name, method.args, ret, method.line
                    ));
                }
            }
        }

        // Top-level functions
        if !self.functions.is_empty() {
            lines.push(String::new());
            lines.push("# Functions".to_string());
            for func in &self.functions {
                let async_prefix = if func.is_async { "async " } else { "" };
                let ret = func
                    .returns
                    .as_ref()
                    .map(|r| format!(" -> {}", r))
                    .unwrap_or_default();

                let decorators = if !func.decorators.is_empty() {
                    format!(
                        " @{}",
                        func.decorators
                            .iter()
                            .take(3)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(",@")
                    )
                } else {
                    String::new()
                };

                lines.push(format!(
                    "{}def {}({}){} L{}{}",
                    async_prefix, func.name, func.args, ret, func.line, decorators
                ));
            }
        }

        // Globals (if any significant ones)
        if !self.globals.is_empty() && self.globals.len() <= 10 {
            lines.push(String::new());
            lines.push(format!("# Globals: {}", self.globals.join(", ")));
        }

        lines.join("\n")
    }

    /// Convert to ultra-condensed string
    pub fn to_ultra_condensed(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("## {}", self.file_path));

        // Imports (just names)
        if !self.imports.is_empty() {
            let imp_names: Vec<String> = self
                .imports
                .iter()
                .filter_map(|imp| imp.name.clone())
                .take(10)
                .map(|n| n.chars().take(10).collect())
                .collect();
            if !imp_names.is_empty() {
                lines.push(format!("imp:{}", imp_names.join(",")));
            }
        }

        // Classes (just names)
        if !self.classes.is_empty() {
            let cls_names: Vec<String> = self
                .classes
                .iter()
                .map(|c| c.name.chars().take(15).collect())
                .collect();
            lines.push(format!("cls:{}", cls_names.join(" ")));
        }

        // Functions (just names)
        let all_funcs: Vec<String> = self
            .functions
            .iter()
            .map(|f| f.name.clone())
            .chain(
                self.classes
                    .iter()
                    .flat_map(|c| c.methods.iter().map(|m| format!("{}.{}", c.name, m.name))),
            )
            .take(20)
            .collect();

        if !all_funcs.is_empty() {
            lines.push(format!("fn:{}", all_funcs.join(" ")));
        }

        lines.join("\n")
    }
}

impl Default for ASTAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let mut analyzer = ASTAnalyzer::new();
        let source = r#"
def hello(name: str) -> str:
    return f"Hello, {name}!"
"#;
        let result = analyzer.analyze(source, "test.py");
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "hello");
        assert_eq!(result.functions[0].args, "name:str");
        assert_eq!(result.functions[0].returns, Some("str".to_string()));
    }

    #[test]
    fn test_parse_class_with_methods() {
        let mut analyzer = ASTAnalyzer::new();
        let source = r#"
class MyClass(Base):
    def __init__(self):
        pass

    def method(self, x: int) -> int:
        return x * 2
"#;
        let result = analyzer.analyze(source, "test.py");
        assert_eq!(result.classes.len(), 1);
        assert_eq!(result.classes[0].name, "MyClass");
        assert_eq!(result.classes[0].bases, vec!["Base"]);
        assert_eq!(result.classes[0].methods.len(), 2);
    }

    #[test]
    fn test_parse_imports() {
        let mut analyzer = ASTAnalyzer::new();
        let source = r#"
import os
from typing import List, Dict
from pathlib import Path as P
"#;
        let result = analyzer.analyze(source, "test.py");
        assert!(!result.imports.is_empty());
    }
}
