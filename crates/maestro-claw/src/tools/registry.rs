//! Tool registry for dynamic tool management
//!
//! The ToolRegistry provides O(1) lookup by name and supports
//! dynamic registration of tools.

use std::collections::HashMap;
use std::sync::Arc;

use super::{Tool, ToolSpec};

/// Registry for managing tools
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tool_names: Vec<&str> = self.tools.keys().map(|s| s.as_str()).collect();
        f.debug_struct("ToolRegistry")
            .field("tools", &tool_names)
            .finish()
    }
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    ///
    /// Returns true if the tool was newly registered,
    /// false if a tool with the same name already exists (duplicate is silently kept).
    pub fn register(&mut self, tool: Arc<dyn Tool>) -> bool {
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            tracing::warn!(
                tool_name = %name,
                "Tool '{}' is already registered; ignoring duplicate registration",
                name
            );
            return false;
        }
        self.tools.insert(name, tool);
        true
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// List all registered tool names
    pub fn list(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Export all tools as provider specs
    pub fn to_tool_specs(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|t| t.to_spec()).collect()
    }

    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Validate that `arguments` satisfies the required-field list declared in
    /// a tool's JSON Schema.  Returns `Ok(())` when all required fields are
    /// present, or `Err(String)` listing the missing fields. (Rec-7)
    pub fn validate_arguments(tool: &dyn Tool, arguments: &serde_json::Value) -> Result<(), String> {
        let schema = tool.parameters_schema();

        let required = match schema.get("required").and_then(|r| r.as_array()) {
            Some(r) => r.clone(),
            None => return Ok(()), // No required fields declared — nothing to check
        };

        let provided = arguments.as_object();
        let missing: Vec<String> = required
            .iter()
            .filter_map(|r| r.as_str())
            .filter(|field| {
                // A field is missing if it is not present in arguments or is null
                match provided {
                    Some(obj) => !obj.contains_key(*field),
                    None => true, // arguments isn't even an object
                }
            })
            .map(|s| s.to_string())
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Missing required argument{}: {}",
                if missing.len() == 1 { "" } else { "s" },
                missing.join(", ")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::Value as JsonValue;

    struct MockTool {
        name: String,
    }

    impl MockTool {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A mock tool"
        }

        fn parameters_schema(&self) -> JsonValue {
            serde_json::json!({"type": "object"})
        }

        async fn execute(&self, _arguments: JsonValue) -> crate::tools::ToolOutput {
            crate::tools::ToolOutput::success("ok".to_string())
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_register() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(MockTool::new("test"));
        assert!(registry.register(tool));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_duplicate_registration() {
        let mut registry = ToolRegistry::new();
        let tool1 = Arc::new(MockTool::new("test"));
        let tool2 = Arc::new(MockTool::new("test"));
        assert!(registry.register(tool1));
        assert!(!registry.register(tool2)); // Duplicate should fail
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_get() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(MockTool::new("test"));
        registry.register(tool);

        let found = registry.get("test");
        assert!(found.is_some());

        let missing = registry.get("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_registry_list() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool::new("tool1")));
        registry.register(Arc::new(MockTool::new("tool2")));

        let mut list = registry.list();
        list.sort();
        assert_eq!(list, vec!["tool1", "tool2"]);
    }

    #[test]
    fn test_registry_to_tool_specs() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool::new("tool1")));
        registry.register(Arc::new(MockTool::new("tool2")));

        let specs = registry.to_tool_specs();
        assert_eq!(specs.len(), 2);
    }

    struct RequiredTool;

    #[async_trait]
    impl Tool for RequiredTool {
        fn name(&self) -> &str { "required" }
        fn description(&self) -> &str { "tool with required fields" }
        fn parameters_schema(&self) -> JsonValue {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"}
                },
                "required": ["path", "content"]
            })
        }
        async fn execute(&self, _arguments: JsonValue) -> crate::tools::ToolOutput {
            crate::tools::ToolOutput::success("ok".to_string())
        }
    }

    #[test]
    fn test_validate_arguments_all_present() {
        let tool = RequiredTool;
        let args = serde_json::json!({"path": "/tmp/x", "content": "hello"});
        assert!(ToolRegistry::validate_arguments(&tool, &args).is_ok());
    }

    #[test]
    fn test_validate_arguments_missing_field() {
        let tool = RequiredTool;
        let args = serde_json::json!({"path": "/tmp/x"}); // content missing
        let result = ToolRegistry::validate_arguments(&tool, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("content"));
    }

    #[test]
    fn test_validate_arguments_no_required_schema() {
        let tool = MockTool::new("no_req");
        let args = serde_json::json!({});
        // MockTool schema has no "required" key — should always pass
        assert!(ToolRegistry::validate_arguments(&tool, &args).is_ok());
    }
}
