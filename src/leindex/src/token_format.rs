//! Token Format Utilities
//!
//! Provides token-efficient string formatting for LLM consumption.

// Unused HashMap removed

/// Token-efficient string formatter
pub struct TokenFormatter;

impl TokenFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Truncate a string to a maximum length with ellipsis
    pub fn truncate(&self, s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            return s.to_string();
        }
        if max_len == 0 {
            return String::new();
        }

        let (target, suffix) = if max_len > 3 {
            (max_len - 3, "...")
        } else {
            (max_len, "")
        };

        let mut out = String::with_capacity(max_len);
        for ch in s.chars() {
            if out.len() + ch.len_utf8() > target {
                break;
            }
            out.push(ch);
        }
        out.push_str(suffix);
        out
    }

    /// Condense/abbreviate a type annotation
    pub fn abbreviate_type(&self, type_str: &str) -> String {
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
            ("Collection", "Coll"),
            ("AbstractSet", "ASet"),
            ("Union", "U"),
            ("Any", "*"),
        ];

        for (full, abbr) in &abbreviations {
            result = result.replace(full, abbr);
        }

        // Truncate generic contents if still too long
        if result.len() > 20 && result.contains('[') {
            if let (Some(start), Some(end)) = (result.find('['), result.rfind(']')) {
                if end > start && end - start > 10 {
                    result = format!("{}[...]", &result[..start]);
                }
            }
        }

        if result.len() > 25 {
            result = self.truncate(&result, 25);
        }

        result
    }

    /// Alias for abbreviate_type
    pub fn condense_type(&self, type_str: &str) -> String {
        self.abbreviate_type(type_str)
    }

    /// Condense function arguments
    pub fn condense_args(&self, args: &str) -> String {
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
                        name.trim().to_string()
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

    /// Format a line number
    pub fn format_line(&self, line: usize) -> String {
        format!("L{}", line)
    }

    /// Format a range of lines
    pub fn format_line_range(&self, start: usize, end: usize) -> String {
        if start == end {
            format!("L{}", start)
        } else {
            format!("L{}-{}", start, end)
        }
    }

    /// Create a summary header
    pub fn create_header(&self, title: &str, count: usize, unit: &str) -> String {
        format!("# {} ({} {})", title, count, unit)
    }

    /// Create a section header
    pub fn create_section(&self, title: &str) -> String {
        format!("## {}", title)
    }

    /// Estimate token count for a string (rough estimate)
    pub fn estimate_tokens(&self, s: &str) -> usize {
        // Rough estimate: ~4 characters per token on average
        s.len().div_ceil(4)
    }

    /// Calculate token savings percentage
    pub fn calculate_savings(&self, original: usize, compressed: usize) -> f64 {
        if original == 0 {
            0.0
        } else {
            (1.0 - compressed as f64 / original as f64) * 100.0
        }
    }
}

impl Default for TokenFormatter {
    fn default() -> Self {
        Self
    }
}

/// Token statistics
#[derive(Debug, Clone)]
pub struct TokenStats {
    pub raw_tokens: usize,
    pub compressed_tokens: usize,
    pub savings_percent: f64,
}

impl TokenStats {
    pub fn new(raw: usize, compressed: usize) -> Self {
        let savings = if raw == 0 {
            0.0
        } else {
            (1.0 - compressed as f64 / raw as f64) * 100.0
        };

        Self {
            raw_tokens: raw,
            compressed_tokens: compressed,
            savings_percent: savings,
        }
    }
}

/// Formatting mode for output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatMode {
    /// Balanced mode - good for most LLM use cases
    Balanced,
    /// Ultra-condensed mode - maximum token savings
    Ultra,
    /// Verbose mode - more details, fewer savings
    Verbose,
}

impl FormatMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ultra" | "u" | "min" | "minimal" => FormatMode::Ultra,
            "verbose" | "v" | "detailed" => FormatMode::Verbose,
            _ => FormatMode::Balanced,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        let formatter = TokenFormatter::new();
        assert_eq!(formatter.truncate("hello", 10), "hello");
        assert_eq!(formatter.truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_condense_type() {
        let formatter = TokenFormatter::new();
        assert_eq!(formatter.condense_type("Optional[str]"), "Opt[str]");
        assert_eq!(
            formatter.condense_type("Callable[[int], int]"),
            "Fn[[int], int]"
        );
    }

    #[test]
    fn test_condense_args() {
        let formatter = TokenFormatter::new();
        assert_eq!(
            formatter.condense_args("self, name: str, age: int = 0"),
            "self, name:str, age:int"
        );
    }

    #[test]
    fn test_token_stats() {
        let stats = TokenStats::new(1000, 50);
        assert!((stats.savings_percent - 95.0).abs() < 0.01);
    }
}
