//! Provider capabilities flags

use serde::{Deserialize, Serialize};

/// Capabilities that a provider supports
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Supports streaming responses
    pub streaming: bool,
    /// Supports native tool calling
    pub native_tools: bool,
    /// Supports vision/image inputs
    pub vision: bool,
    /// Supports function calling
    pub function_calling: bool,
    /// Supports system messages
    pub system_messages: bool,
    /// Supports parallel tool calls
    pub parallel_tool_calls: bool,
}

impl ProviderCapabilities {
    /// Create capabilities with all features disabled
    pub fn none() -> Self {
        Self {
            streaming: false,
            native_tools: false,
            vision: false,
            function_calling: false,
            system_messages: false,
            parallel_tool_calls: false,
        }
    }

    /// Create capabilities with all features enabled
    pub fn all() -> Self {
        Self {
            streaming: true,
            native_tools: true,
            vision: true,
            function_calling: true,
            system_messages: true,
            parallel_tool_calls: true,
        }
    }

    /// OpenAI capabilities (GPT-4, GPT-3.5)
    pub fn openai() -> Self {
        Self {
            streaming: true,
            native_tools: true,
            vision: true,
            function_calling: true,
            system_messages: true,
            parallel_tool_calls: true,
        }
    }

    /// Anthropic capabilities (Claude)
    pub fn anthropic() -> Self {
        Self {
            streaming: true,
            native_tools: true,
            vision: true,
            function_calling: true,
            system_messages: true,
            parallel_tool_calls: true,
        }
    }

    /// Ollama capabilities (local models)
    ///
    /// LOW-9: `native_tools` defaults to `false` because the vast majority of
    /// Ollama-served models do not support the Ollama tools API. Callers that
    /// have confirmed their model supports native tools can override this field.
    pub fn ollama() -> Self {
        Self {
            streaming: true,
            native_tools: false,
            vision: false,
            function_calling: false,
            system_messages: true,
            parallel_tool_calls: false,
        }
    }

    /// OpenRouter capabilities (depends on model)
    pub fn openrouter() -> Self {
        Self {
            streaming: true,
            native_tools: true,
            vision: true,
            function_calling: true,
            system_messages: true,
            parallel_tool_calls: true,
        }
    }
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self::openai()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_none() {
        let caps = ProviderCapabilities::none();
        assert!(!caps.streaming);
        assert!(!caps.native_tools);
        assert!(!caps.vision);
    }

    #[test]
    fn test_capabilities_all() {
        let caps = ProviderCapabilities::all();
        assert!(caps.streaming);
        assert!(caps.native_tools);
        assert!(caps.vision);
    }

    #[test]
    fn test_capabilities_openai() {
        let caps = ProviderCapabilities::openai();
        assert!(caps.streaming);
        assert!(caps.native_tools);
        assert!(caps.vision);
    }

    #[test]
    fn test_capabilities_anthropic() {
        let caps = ProviderCapabilities::anthropic();
        assert!(caps.streaming);
        assert!(caps.native_tools);
        assert!(caps.vision);
    }

    #[test]
    fn test_capabilities_ollama() {
        let caps = ProviderCapabilities::ollama();
        assert!(caps.streaming);
        // LOW-9: Ollama native_tools must default to false (most models don't support it)
        assert!(!caps.native_tools);
        assert!(!caps.parallel_tool_calls);
    }

    #[test]
    fn test_capabilities_serialization() {
        let caps = ProviderCapabilities::openai();
        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("streaming"));
    }

    #[test]
    fn test_capabilities_deserialization() {
        let json = r#"{"streaming":true,"native_tools":false,"vision":false,"function_calling":true,"system_messages":true,"parallel_tool_calls":false}"#;
        let caps: ProviderCapabilities = serde_json::from_str(json).unwrap();
        assert!(caps.streaming);
        assert!(!caps.native_tools);
    }
}
