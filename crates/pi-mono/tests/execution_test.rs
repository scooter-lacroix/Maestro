//! Tests for the execution module
//!
//! This test suite validates the enhanced execution result structures
//! following TDD principles.

use maestro_pi_mono::execution::{
    ExecutionResult, Executor, ExecutorConfig, StreamEvent, StreamEventType, SubagentResult,
    UsageMetrics,
};
use std::time::Duration;

#[test]
fn test_usage_metrics_creation() {
    let metrics = UsageMetrics {
        tokens_input: 1000,
        tokens_output: 500,
        tokens_total: 1500,
        cost_estimate_usd: Some(0.003),
        duration: Duration::from_secs(10),
    };

    assert_eq!(metrics.tokens_input, 1000);
    assert_eq!(metrics.tokens_output, 500);
    assert_eq!(metrics.tokens_total, 1500);
    assert_eq!(metrics.cost_estimate_usd, Some(0.003));
    assert_eq!(metrics.duration, Duration::from_secs(10));
}

#[test]
fn test_usage_metrics_new() {
    let metrics = UsageMetrics::new(2000, 1000, Some(0.006), Duration::from_secs(20));

    assert_eq!(metrics.tokens_input, 2000);
    assert_eq!(metrics.tokens_output, 1000);
    assert_eq!(metrics.tokens_total, 3000); // Automatically calculated
    assert_eq!(metrics.cost_estimate_usd, Some(0.006));
    assert_eq!(metrics.duration, Duration::from_secs(20));
}

#[test]
fn test_usage_metrics_cost_per_million_tokens() {
    let metrics = UsageMetrics::new(1000, 500, Some(0.003), Duration::from_secs(10));

    // Cost per million = (0.003 / 1500) * 1,000,000 = 2.0
    let cost_per_million = metrics.cost_per_million_tokens();
    assert_eq!(cost_per_million, Some(2.0));
}

#[test]
fn test_usage_metrics_cost_per_million_tokens_no_cost() {
    let metrics = UsageMetrics::new(1000, 500, None, Duration::from_secs(10));

    assert_eq!(metrics.cost_per_million_tokens(), None);
}

#[test]
fn test_usage_metrics_tokens_per_second() {
    let metrics = UsageMetrics::new(1000, 500, Some(0.003), Duration::from_secs(10));

    // 1500 tokens / 10 seconds = 150 tokens per second
    let tps = metrics.tokens_per_second();
    assert_eq!(tps, 150.0);
}

#[test]
fn test_usage_metrics_tokens_per_second_zero_duration() {
    let metrics = UsageMetrics::new(1000, 500, Some(0.003), Duration::from_secs(0));

    // Should return 0.0 when duration is 0 to avoid division by zero
    let tps = metrics.tokens_per_second();
    assert_eq!(tps, 0.0);
}

#[test]
fn test_usage_metrics_serialization() {
    let metrics = UsageMetrics::new(1000, 500, Some(0.003), Duration::from_secs(10));

    // Test serialization to JSON
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(json.contains("\"tokens_input\":1000"));
    assert!(json.contains("\"tokens_output\":500"));
    assert!(json.contains("\"tokens_total\":1500"));

    // Test deserialization
    let deserialized: UsageMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.tokens_input, metrics.tokens_input);
    assert_eq!(deserialized.tokens_output, metrics.tokens_output);
    assert_eq!(deserialized.tokens_total, metrics.tokens_total);
}

#[test]
fn test_stream_event_type_variants() {
    let start = StreamEventType::Start;
    let progress = StreamEventType::Progress;
    let data = StreamEventType::Data;
    let error = StreamEventType::Error;
    let complete = StreamEventType::Complete;

    assert_eq!(start, StreamEventType::Start);
    assert_eq!(progress, StreamEventType::Progress);
    assert_eq!(data, StreamEventType::Data);
    assert_eq!(error, StreamEventType::Error);
    assert_eq!(complete, StreamEventType::Complete);

    // Test inequality
    assert_ne!(start, progress);
    assert_ne!(progress, data);
    assert_ne!(data, error);
    assert_ne!(error, complete);
}

#[test]
fn test_stream_event_creation() {
    use std::time::SystemTime;

    let event = StreamEvent {
        timestamp: SystemTime::now(),
        event_type: StreamEventType::Progress,
        content: "Processing...".to_string(),
        metadata: Some("50%".to_string()),
    };

    assert_eq!(event.event_type, StreamEventType::Progress);
    assert_eq!(event.content, "Processing...");
    assert_eq!(event.metadata, Some("50%".to_string()));
}

#[test]
fn test_stream_event_new() {
    let event = StreamEvent::new(
        StreamEventType::Progress,
        "Processing...".to_string(),
        Some("50%".to_string()),
    );

    assert_eq!(event.event_type, StreamEventType::Progress);
    assert_eq!(event.content, "Processing...");
    assert_eq!(event.metadata, Some("50%".to_string()));
}

#[test]
fn test_stream_event_start() {
    let event = StreamEvent::start("Starting task...".to_string());

    assert_eq!(event.event_type, StreamEventType::Start);
    assert_eq!(event.content, "Starting task...");
    assert!(event.metadata.is_none());
}

#[test]
fn test_stream_event_progress() {
    let event = StreamEvent::progress("50% complete".to_string(), Some("50".to_string()));

    assert_eq!(event.event_type, StreamEventType::Progress);
    assert_eq!(event.content, "50% complete");
    assert_eq!(event.metadata, Some("50".to_string()));
}

#[test]
fn test_stream_event_progress_no_metadata() {
    let event = StreamEvent::progress("Processing...".to_string(), None);

    assert_eq!(event.event_type, StreamEventType::Progress);
    assert_eq!(event.content, "Processing...");
    assert!(event.metadata.is_none());
}

#[test]
fn test_stream_event_data() {
    let event = StreamEvent::data("Received data chunk".to_string());

    assert_eq!(event.event_type, StreamEventType::Data);
    assert_eq!(event.content, "Received data chunk");
    assert!(event.metadata.is_none());
}

#[test]
fn test_stream_event_error() {
    let event = StreamEvent::error("Connection failed".to_string());

    assert_eq!(event.event_type, StreamEventType::Error);
    assert_eq!(event.content, "Connection failed");
    assert!(event.metadata.is_none());
}

#[test]
fn test_stream_event_complete() {
    let event = StreamEvent::complete("Task finished".to_string());

    assert_eq!(event.event_type, StreamEventType::Complete);
    assert_eq!(event.content, "Task finished");
    assert!(event.metadata.is_none());
}

#[test]
fn test_stream_event_serialization() {
    let event = StreamEvent::new(
        StreamEventType::Progress,
        "Processing...".to_string(),
        Some("50%".to_string()),
    );

    // Test serialization to JSON
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("Progress"));
    assert!(json.contains("Processing..."));

    // Test deserialization
    let deserialized: StreamEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.event_type, event.event_type);
    assert_eq!(deserialized.content, event.content);
    assert_eq!(deserialized.metadata, event.metadata);
}

#[test]
fn test_subagent_result_success() {
    let result = SubagentResult::success(
        "Analyze code".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Analysis complete".to_string(),
        Duration::from_secs(5),
    );

    assert!(result.is_success());
    assert!(!result.is_failure());
    assert_eq!(result.task, "Analyze code");
    assert_eq!(result.agent, "agent-001");
    assert_eq!(result.agent_type, "analyzer");
    assert_eq!(result.output, "Analysis complete");
    assert!(result.error.is_none());
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.duration, Duration::from_secs(5));
    assert!(result.usage.is_none());
    assert_eq!(result.events.len(), 0);
}

#[test]
fn test_subagent_result_failure() {
    let result = SubagentResult::failure(
        "Analyze code".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Timeout error".to_string(),
        Duration::from_secs(10),
    );

    assert!(!result.is_success());
    assert!(result.is_failure());
    assert_eq!(result.task, "Analyze code");
    assert_eq!(result.agent, "agent-001");
    assert_eq!(result.agent_type, "analyzer");
    assert!(result.output.is_empty());
    assert_eq!(result.error, Some("Timeout error".to_string()));
    assert!(result.exit_code.is_none());
    assert_eq!(result.duration, Duration::from_secs(10));
    assert!(result.usage.is_none());
    assert_eq!(result.events.len(), 0);
}

#[test]
fn test_subagent_result_with_usage() {
    let usage = UsageMetrics::new(1000, 500, Some(0.003), Duration::from_secs(5));

    let result = SubagentResult::success(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    )
    .with_usage(usage.clone());

    assert!(result.usage.is_some());
    let result_usage = result.usage.as_ref().unwrap();
    assert_eq!(result_usage.tokens_input, 1000);
    assert_eq!(result_usage.tokens_output, 500);
    assert_eq!(result_usage.tokens_total, 1500);
}

#[test]
fn test_subagent_result_with_event() {
    let event = StreamEvent::start("Starting".to_string());

    let result = SubagentResult::success(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    )
    .with_event(event.clone());

    assert_eq!(result.events.len(), 1);
    assert_eq!(result.events[0].event_type, StreamEventType::Start);
    assert_eq!(result.events[0].content, "Starting");
}

#[test]
fn test_subagent_result_with_multiple_events() {
    let result = SubagentResult::success(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    )
    .with_event(StreamEvent::start("Starting".to_string()))
    .with_event(StreamEvent::progress(
        "50%".to_string(),
        Some("50".to_string()),
    ))
    .with_event(StreamEvent::complete("Done".to_string()));

    assert_eq!(result.events.len(), 3);
    assert_eq!(result.events[0].event_type, StreamEventType::Start);
    assert_eq!(result.events[1].event_type, StreamEventType::Progress);
    assert_eq!(result.events[2].event_type, StreamEventType::Complete);
}

#[test]
fn test_subagent_result_is_success() {
    let success_result = SubagentResult::success(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    );

    let failure_result = SubagentResult::failure(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Error".to_string(),
        Duration::from_secs(5),
    );

    assert!(success_result.is_success());
    assert!(!failure_result.is_success());
}

#[test]
fn test_subagent_result_is_failure() {
    let success_result = SubagentResult::success(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    );

    let failure_result = SubagentResult::failure(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Error".to_string(),
        Duration::from_secs(5),
    );

    assert!(!success_result.is_failure());
    assert!(failure_result.is_failure());
}

#[test]
fn test_subagent_result_event_count() {
    let result = SubagentResult::success(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    );

    assert_eq!(result.event_count(), 0);

    let result = result
        .with_event(StreamEvent::start("Starting".to_string()))
        .with_event(StreamEvent::complete("Done".to_string()));

    assert_eq!(result.event_count(), 2);
}

#[test]
fn test_subagent_result_error_events() {
    let result = SubagentResult::success(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    )
    .with_event(StreamEvent::start("Starting".to_string()))
    .with_event(StreamEvent::error("Warning".to_string()))
    .with_event(StreamEvent::data("Data".to_string()))
    .with_event(StreamEvent::error("Critical error".to_string()));

    let errors = result.error_events();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].content, "Warning");
    assert_eq!(errors[1].content, "Critical error");
}

#[test]
fn test_subagent_result_error_events_empty() {
    let result = SubagentResult::success(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    )
    .with_event(StreamEvent::start("Starting".to_string()))
    .with_event(StreamEvent::data("Data".to_string()));

    let errors = result.error_events();
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_subagent_result_summary() {
    let success_result = SubagentResult::success(
        "Analyze code".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    );

    let summary = success_result.summary();
    assert!(summary.contains("[SUCCESS]"));
    assert!(summary.contains("Analyze code"));
    assert!(summary.contains("agent-001"));
    assert!(summary.contains("analyzer"));

    let failure_result = SubagentResult::failure(
        "Fix bug".to_string(),
        "agent-002".to_string(),
        "coder".to_string(),
        "Error".to_string(),
        Duration::from_secs(10),
    );

    let summary = failure_result.summary();
    assert!(summary.contains("[FAILURE]"));
    assert!(summary.contains("Fix bug"));
    assert!(summary.contains("agent-002"));
    assert!(summary.contains("coder"));
}

#[test]
fn test_subagent_result_serialization() {
    let result = SubagentResult::success(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    )
    .with_usage(UsageMetrics::new(
        1000,
        500,
        Some(0.003),
        Duration::from_secs(5),
    ))
    .with_event(StreamEvent::start("Starting".to_string()));

    // Test serialization to JSON
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("Task"));
    assert!(json.contains("agent-001"));

    // Test deserialization
    let deserialized: SubagentResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.success, result.success);
    assert_eq!(deserialized.task, result.task);
    assert_eq!(deserialized.agent, result.agent);
    assert_eq!(deserialized.agent_type, result.agent_type);
}

#[test]
fn test_subagent_result_builder_pattern() {
    let usage = UsageMetrics::new(2000, 1000, Some(0.006), Duration::from_secs(10));

    let result = SubagentResult::success(
        "Complex task".to_string(),
        "agent-003".to_string(),
        "reviewer".to_string(),
        "Review complete".to_string(),
        Duration::from_secs(15),
    )
    .with_usage(usage)
    .with_event(StreamEvent::start("Starting review".to_string()))
    .with_event(StreamEvent::progress(
        "50%".to_string(),
        Some("50".to_string()),
    ))
    .with_event(StreamEvent::complete("Review done".to_string()));

    assert!(result.is_success());
    assert!(result.usage.is_some());
    assert_eq!(result.event_count(), 3);
    assert_eq!(result.usage.unwrap().tokens_total, 3000);
}

#[test]
fn test_existing_execution_result_still_works() {
    // Ensure we didn't break existing functionality
    let success = ExecutionResult::success("Operation completed".to_string());
    assert!(success.success);
    assert_eq!(success.output, "Operation completed");
    assert!(success.error.is_none());

    let failure = ExecutionResult::failure("Connection failed".to_string());
    assert!(!failure.success);
    assert!(failure.output.is_empty());
    assert_eq!(failure.error, Some("Connection failed".to_string()));
}

#[test]
fn test_existing_executor_config_still_works() {
    let config = ExecutorConfig {
        timeout_secs: 600,
        max_retries: 5,
    };
    assert_eq!(config.timeout_secs, 600);
    assert_eq!(config.max_retries, 5);

    let default_config = ExecutorConfig::default();
    assert_eq!(default_config.timeout_secs, 300);
    assert_eq!(default_config.max_retries, 3);
}

#[test]
fn test_existing_executor_still_works() {
    let executor = Executor::default();
    // Just ensure it compiles and creates
    let _ = executor;

    let custom_executor = Executor::new(ExecutorConfig {
        timeout_secs: 600,
        max_retries: 5,
    });
    let _ = custom_executor;
}

#[test]
fn test_subagent_result_clone() {
    let result = SubagentResult::success(
        "Task".to_string(),
        "agent-001".to_string(),
        "analyzer".to_string(),
        "Done".to_string(),
        Duration::from_secs(5),
    )
    .with_usage(UsageMetrics::new(
        1000,
        500,
        Some(0.003),
        Duration::from_secs(5),
    ))
    .with_event(StreamEvent::start("Starting".to_string()));

    let cloned = result.clone();

    assert_eq!(cloned.task, result.task);
    assert_eq!(cloned.agent, result.agent);
    assert_eq!(cloned.agent_type, result.agent_type);
    assert_eq!(cloned.events.len(), result.events.len());
    assert!(cloned.usage.is_some());
}

#[test]
fn test_usage_metrics_clone() {
    let metrics = UsageMetrics::new(1000, 500, Some(0.003), Duration::from_secs(10));

    let cloned = metrics.clone();

    assert_eq!(cloned.tokens_input, metrics.tokens_input);
    assert_eq!(cloned.tokens_output, metrics.tokens_output);
    assert_eq!(cloned.tokens_total, metrics.tokens_total);
    assert_eq!(cloned.cost_estimate_usd, metrics.cost_estimate_usd);
    assert_eq!(cloned.duration, metrics.duration);
}

#[test]
fn test_stream_event_clone() {
    let event = StreamEvent::new(
        StreamEventType::Progress,
        "Processing...".to_string(),
        Some("50%".to_string()),
    );

    let cloned = event.clone();

    assert_eq!(cloned.event_type, event.event_type);
    assert_eq!(cloned.content, event.content);
    assert_eq!(cloned.metadata, event.metadata);
}
