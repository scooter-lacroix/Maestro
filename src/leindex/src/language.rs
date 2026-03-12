//! Language Abstraction Layer
//!
//! Provides unified multi-language parsing using tree-sitter.
//! Supports: Python, JavaScript, TypeScript, Rust, Go, Java, C, C++

use serde::{Deserialize, Serialize};
// Unused HashSet removed
use std::path::Path;
use tree_sitter::{Language, Node, Parser, Tree};

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProgrammingLanguage {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Go,
    Java,
    C,
    Cpp,
}

impl ProgrammingLanguage {
    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "py" | "pyw" | "pyi" => Some(Self::Python),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Self::TypeScript),
            "rs" => Some(Self::Rust),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Some(Self::Cpp),
            _ => None,
        }
    }

    /// Detect language from file path
    pub fn from_path(path: &str) -> Option<Self> {
        Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }

    /// Get the tree-sitter language for this programming language
    pub fn tree_sitter_language(&self) -> Language {
        match self {
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::C => "C",
            Self::Cpp => "C++",
        }
    }

    /// Get all supported languages
    pub fn all() -> &'static [Self] {
        &[
            Self::Python,
            Self::JavaScript,
            Self::TypeScript,
            Self::Rust,
            Self::Go,
            Self::Java,
            Self::C,
            Self::Cpp,
        ]
    }
}

/// Universal parsed code element types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeElement {
    Import(ImportElement),
    Function(FunctionElement),
    Class(ClassElement),
    Variable(VariableElement),
}

/// Import/dependency element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportElement {
    pub module: String,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub line: usize,
    pub is_default: bool,
}

/// Function/method element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionElement {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    pub params: Vec<ParameterElement>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_method: bool,
    pub is_static: bool,
    pub visibility: Visibility,
    pub decorators: Vec<String>,
    pub class_name: Option<String>,
}

/// Function parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterElement {
    pub name: String,
    pub type_hint: Option<String>,
    pub has_default: bool,
    pub is_variadic: bool,
}

/// Class/struct/interface element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassElement {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    pub bases: Vec<String>,
    pub interfaces: Vec<String>,
    pub is_interface: bool,
    pub is_abstract: bool,
    pub visibility: Visibility,
    pub decorators: Vec<String>,
    pub methods: Vec<FunctionElement>,
    pub fields: Vec<VariableElement>,
}

/// Variable/field element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableElement {
    pub name: String,
    pub line: usize,
    pub type_hint: Option<String>,
    pub is_const: bool,
    pub visibility: Visibility,
}

/// Visibility/access modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum Visibility {
    #[default]
    Public,
    Private,
    Protected,
    Package, // Go package-level, Java package-private
}


/// Multi-language parser using tree-sitter
pub struct MultiLanguageParser {
    parser: Parser,
    current_language: Option<ProgrammingLanguage>,
}

impl MultiLanguageParser {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            current_language: None,
        }
    }

    /// Set the language for parsing
    pub fn set_language(&mut self, language: ProgrammingLanguage) -> Result<(), String> {
        self.parser
            .set_language(&language.tree_sitter_language())
            .map_err(|e| format!("Failed to set language: {}", e))?;
        self.current_language = Some(language);
        Ok(())
    }

    /// Parse source code and return the syntax tree
    pub fn parse(&mut self, source: &str, language: ProgrammingLanguage) -> Option<Tree> {
        if self.current_language != Some(language) && self.set_language(language).is_err() {
            return None;
        }
        self.parser.parse(source, None)
    }

    /// Get current language
    pub fn current_language(&self) -> Option<ProgrammingLanguage> {
        self.current_language
    }
}

impl Default for MultiLanguageParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Language-specific configuration for analysis
pub trait LanguageConfig {
    /// Get function definition node types
    fn function_node_types(&self) -> &'static [&'static str];

    /// Get class/struct definition node types
    fn class_node_types(&self) -> &'static [&'static str];

    /// Get import statement node types
    fn import_node_types(&self) -> &'static [&'static str];

    /// Get control flow node types (if, for, while, etc.)
    fn control_flow_node_types(&self) -> &'static [&'static str];

    /// Get assignment node types
    fn assignment_node_types(&self) -> &'static [&'static str];

    /// Get comment node types
    fn comment_node_types(&self) -> &'static [&'static str];

    /// Check if a name indicates a private member
    fn is_private_name(&self, name: &str) -> bool;
}

/// Python language configuration
pub struct PythonConfig;

impl LanguageConfig for PythonConfig {
    fn function_node_types(&self) -> &'static [&'static str] {
        &["function_definition"]
    }

    fn class_node_types(&self) -> &'static [&'static str] {
        &["class_definition"]
    }

    fn import_node_types(&self) -> &'static [&'static str] {
        &["import_statement", "import_from_statement"]
    }

    fn control_flow_node_types(&self) -> &'static [&'static str] {
        &[
            "if_statement",
            "elif_clause",
            "else_clause",
            "for_statement",
            "while_statement",
            "try_statement",
            "except_clause",
            "finally_clause",
            "with_statement",
            "match_statement",
            "case_clause",
        ]
    }

    fn assignment_node_types(&self) -> &'static [&'static str] {
        &["assignment", "augmented_assignment"]
    }

    fn comment_node_types(&self) -> &'static [&'static str] {
        &["comment"]
    }

    fn is_private_name(&self, name: &str) -> bool {
        name.starts_with('_') && !name.starts_with("__")
    }
}

/// JavaScript/TypeScript language configuration
pub struct JavaScriptConfig;

impl LanguageConfig for JavaScriptConfig {
    fn function_node_types(&self) -> &'static [&'static str] {
        &[
            "function_declaration",
            "function_expression",
            "arrow_function",
            "method_definition",
            "generator_function_declaration",
        ]
    }

    fn class_node_types(&self) -> &'static [&'static str] {
        &["class_declaration", "class"]
    }

    fn import_node_types(&self) -> &'static [&'static str] {
        &["import_statement", "export_statement"]
    }

    fn control_flow_node_types(&self) -> &'static [&'static str] {
        &[
            "if_statement",
            "else_clause",
            "for_statement",
            "for_in_statement",
            "while_statement",
            "do_statement",
            "switch_statement",
            "switch_case",
            "try_statement",
            "catch_clause",
            "finally_clause",
        ]
    }

    fn assignment_node_types(&self) -> &'static [&'static str] {
        &[
            "assignment_expression",
            "augmented_assignment_expression",
            "variable_declaration",
            "lexical_declaration",
        ]
    }

    fn comment_node_types(&self) -> &'static [&'static str] {
        &["comment"]
    }

    fn is_private_name(&self, name: &str) -> bool {
        name.starts_with('#') || name.starts_with('_')
    }
}

/// Rust language configuration
pub struct RustConfig;

impl LanguageConfig for RustConfig {
    fn function_node_types(&self) -> &'static [&'static str] {
        &["function_item"]
    }

    fn class_node_types(&self) -> &'static [&'static str] {
        &["struct_item", "enum_item", "trait_item", "impl_item"]
    }

    fn import_node_types(&self) -> &'static [&'static str] {
        &["use_declaration", "extern_crate_declaration"]
    }

    fn control_flow_node_types(&self) -> &'static [&'static str] {
        &[
            "if_expression",
            "else_clause",
            "for_expression",
            "while_expression",
            "loop_expression",
            "match_expression",
            "match_arm",
        ]
    }

    fn assignment_node_types(&self) -> &'static [&'static str] {
        &[
            "let_declaration",
            "assignment_expression",
            "compound_assignment_expr",
        ]
    }

    fn comment_node_types(&self) -> &'static [&'static str] {
        &["line_comment", "block_comment"]
    }

    fn is_private_name(&self, _name: &str) -> bool {
        // In Rust, visibility is explicit via `pub`, not by name
        false
    }
}

/// Go language configuration
pub struct GoConfig;

impl LanguageConfig for GoConfig {
    fn function_node_types(&self) -> &'static [&'static str] {
        &["function_declaration", "method_declaration"]
    }

    fn class_node_types(&self) -> &'static [&'static str] {
        &["type_declaration", "type_spec"] // struct, interface
    }

    fn import_node_types(&self) -> &'static [&'static str] {
        &["import_declaration", "import_spec"]
    }

    fn control_flow_node_types(&self) -> &'static [&'static str] {
        &[
            "if_statement",
            "for_statement",
            "switch_statement",
            "select_statement",
            "expression_case",
            "default_case",
        ]
    }

    fn assignment_node_types(&self) -> &'static [&'static str] {
        &[
            "short_var_declaration",
            "assignment_statement",
            "var_declaration",
        ]
    }

    fn comment_node_types(&self) -> &'static [&'static str] {
        &["comment"]
    }

    fn is_private_name(&self, name: &str) -> bool {
        // In Go, lowercase first letter means package-private
        name.chars().next().is_some_and(|c| c.is_lowercase())
    }
}

/// Java language configuration
pub struct JavaConfig;

impl LanguageConfig for JavaConfig {
    fn function_node_types(&self) -> &'static [&'static str] {
        &["method_declaration", "constructor_declaration"]
    }

    fn class_node_types(&self) -> &'static [&'static str] {
        &[
            "class_declaration",
            "interface_declaration",
            "enum_declaration",
            "annotation_type_declaration",
        ]
    }

    fn import_node_types(&self) -> &'static [&'static str] {
        &["import_declaration"]
    }

    fn control_flow_node_types(&self) -> &'static [&'static str] {
        &[
            "if_statement",
            "else",
            "for_statement",
            "enhanced_for_statement",
            "while_statement",
            "do_statement",
            "switch_expression",
            "switch_label",
            "try_statement",
            "catch_clause",
            "finally_clause",
        ]
    }

    fn assignment_node_types(&self) -> &'static [&'static str] {
        &[
            "assignment_expression",
            "local_variable_declaration",
            "field_declaration",
        ]
    }

    fn comment_node_types(&self) -> &'static [&'static str] {
        &["line_comment", "block_comment"]
    }

    fn is_private_name(&self, _name: &str) -> bool {
        // Java uses explicit access modifiers
        false
    }
}

/// C/C++ language configuration
pub struct CppConfig;

impl LanguageConfig for CppConfig {
    fn function_node_types(&self) -> &'static [&'static str] {
        &["function_definition", "function_declarator"]
    }

    fn class_node_types(&self) -> &'static [&'static str] {
        &["struct_specifier", "class_specifier", "enum_specifier"]
    }

    fn import_node_types(&self) -> &'static [&'static str] {
        &["preproc_include", "preproc_import"]
    }

    fn control_flow_node_types(&self) -> &'static [&'static str] {
        &[
            "if_statement",
            "else_clause",
            "for_statement",
            "for_range_loop",
            "while_statement",
            "do_statement",
            "switch_statement",
            "case_statement",
            "try_statement",
            "catch_clause",
        ]
    }

    fn assignment_node_types(&self) -> &'static [&'static str] {
        &["assignment_expression", "declaration", "init_declarator"]
    }

    fn comment_node_types(&self) -> &'static [&'static str] {
        &["comment"]
    }

    fn is_private_name(&self, _name: &str) -> bool {
        // C++ uses explicit access specifiers
        false
    }
}

/// Get language configuration for a programming language
pub fn get_language_config(lang: ProgrammingLanguage) -> Box<dyn LanguageConfig> {
    match lang {
        ProgrammingLanguage::Python => Box::new(PythonConfig),
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            Box::new(JavaScriptConfig)
        }
        ProgrammingLanguage::Rust => Box::new(RustConfig),
        ProgrammingLanguage::Go => Box::new(GoConfig),
        ProgrammingLanguage::Java => Box::new(JavaConfig),
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => Box::new(CppConfig),
    }
}

/// Extract text content from a tree-sitter node
pub fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    if end <= source.len() && start < end {
        &source[start..end]
    } else {
        ""
    }
}

/// Find child node by field name
pub fn child_by_field<'a>(node: Node<'a>, field: &str) -> Option<Node<'a>> {
    node.child_by_field_name(field)
}

/// Find all children matching a node type
pub fn children_by_type<'a>(node: Node<'a>, node_type: &str) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    let mut results = Vec::new();

    for child in node.children(&mut cursor) {
        if child.kind() == node_type {
            results.push(child);
        }
    }

    results
}

/// Recursively find all nodes of a given type
pub fn find_all_nodes<'a>(node: Node<'a>, node_types: &[&str]) -> Vec<Node<'a>> {
    let mut results = Vec::new();
    let _cursor = node.walk();

    fn visit<'a>(node: Node<'a>, node_types: &[&str], results: &mut Vec<Node<'a>>) {
        if node_types.contains(&node.kind()) {
            results.push(node);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            visit(child, node_types, results);
        }
    }

    visit(node, node_types, &mut results);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(
            ProgrammingLanguage::from_extension("py"),
            Some(ProgrammingLanguage::Python)
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("js"),
            Some(ProgrammingLanguage::JavaScript)
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("ts"),
            Some(ProgrammingLanguage::TypeScript)
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("rs"),
            Some(ProgrammingLanguage::Rust)
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("go"),
            Some(ProgrammingLanguage::Go)
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("java"),
            Some(ProgrammingLanguage::Java)
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("c"),
            Some(ProgrammingLanguage::C)
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("cpp"),
            Some(ProgrammingLanguage::Cpp)
        );
        assert_eq!(ProgrammingLanguage::from_extension("unknown"), None);
    }

    #[test]
    fn test_path_detection() {
        assert_eq!(
            ProgrammingLanguage::from_path("/path/to/file.py"),
            Some(ProgrammingLanguage::Python)
        );
        assert_eq!(
            ProgrammingLanguage::from_path("module.ts"),
            Some(ProgrammingLanguage::TypeScript)
        );
        assert_eq!(
            ProgrammingLanguage::from_path("main.rs"),
            Some(ProgrammingLanguage::Rust)
        );
    }

    #[test]
    fn test_parser_creation() {
        let mut parser = MultiLanguageParser::new();
        assert!(parser.set_language(ProgrammingLanguage::Python).is_ok());
        assert_eq!(parser.current_language(), Some(ProgrammingLanguage::Python));
    }

    #[test]
    fn test_parse_python() {
        let mut parser = MultiLanguageParser::new();
        let source = "def hello():\n    print('hello')";
        let tree = parser.parse(source, ProgrammingLanguage::Python);
        assert!(tree.is_some());
    }

    #[test]
    fn test_parse_javascript() {
        let mut parser = MultiLanguageParser::new();
        let source = "function hello() { console.log('hello'); }";
        let tree = parser.parse(source, ProgrammingLanguage::JavaScript);
        assert!(tree.is_some());
    }

    #[test]
    fn test_parse_rust() {
        let mut parser = MultiLanguageParser::new();
        let source = "fn hello() { println!(\"hello\"); }";
        let tree = parser.parse(source, ProgrammingLanguage::Rust);
        assert!(tree.is_some());
    }
}
