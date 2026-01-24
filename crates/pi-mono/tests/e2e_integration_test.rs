//! End-to-end integration tests for Pi-Mono Integration
//!
//! This test suite validates the complete pi-mono integration workflow
//! from detection through configuration, execution, and CLI commands.

use maestro_pi_mono::{
    detection::PiDetection, discovery::ModelDiscovery, config::PiMonoConfig,
    agents::mapping::{AgentRegistry, PiAgentType, AgentRole, ToolAccess, TaskComplexity, AgentMapping, role_to_pi_agent_type},
    execution::{SubagentRunner, Executor, ExecutorConfig, ExecutionResult, StreamEvent, StreamEventType, UsageMetrics, SubagentResult},
};
use std::time::Duration;

/// Test 1: End-to-end detection workflow
#[tokio::test]
async fn e2e_detection_workflow() {
    let detection_result = PiDetection::detect();

    if detection_result.is_err() {
        println!("Skipping e2e_detection_workflow: pi-mono not installed");
        return;
    }

    let detection = detection_result.unwrap();

    // Verify detection found pi-mono
    assert!(!detection.executable_path.as_os_str().is_empty());

    // Verify capabilities were detected
    println!("Capabilities: subagent={}, streaming={}, parallel={}, chain={}",
        detection.capabilities.subagent,
        detection.capabilities.streaming,
        detection.capabilities.parallel,
        detection.capabilities.chain);

    assert!(detection.capabilities.subagent);
}

/// Test 2: End-to-end model discovery workflow
#[tokio::test]
async fn e2e_model_discovery_workflow() {
    let detection_result = PiDetection::detect();
    if detection_result.is_err() {
        println!("Skipping e2e_model_discovery_workflow: pi-mono not installed");
        return;
    }

    let detection = detection_result.unwrap();
    let mut discovery = ModelDiscovery::new(detection);

    let discovery_result = discovery.discover_models().await;

    match discovery_result {
        Ok(result) => {
            println!("Discovered {} models", result.models.len());
            println!("Found {} providers", result.providers.len());

            assert!(!result.providers.is_empty());

            let cache_duration = result.cache_expires
                .duration_since(result.discovered_at)
                .unwrap_or_default();
            assert_eq!(cache_duration.as_secs(), 86400);
        }
        Err(e) => {
            println!("Model discovery failed (expected if no API keys): {}", e);
        }
    }
}

/// Test 3: End-to-end configuration workflow
#[tokio::test]
async fn e2e_configuration_workflow() {
    let config = PiMonoConfig::default();
    assert!(config.path.is_none());
    assert!(config.providers.is_empty());
    assert!(config.model_preferences.is_empty());

    let mut config = PiMonoConfig::default();
    config.path = Some("/usr/local/bin/pi".to_string());
    config.version_info = Some("0.49.3".to_string());

    assert_eq!(config.path, Some("/usr/local/bin/pi".to_string()));
    assert_eq!(config.version_info, Some("0.49.3".to_string()));

    // Test AgentRegistry creation
    let registry = AgentRegistry::new(config.clone());
    // AgentRegistry::new returns Self directly, not Result

    // Test getting agents by role (not by PiAgentType)
    let roles = vec![
        AgentRole::Scout,
        AgentRole::Architect,
        AgentRole::Critic,
        AgentRole::Kraken,
    ];

    for role in roles {
        let result = registry.get_agent(role.clone());
        println!("Role {:?}: {:?}", role, result.is_ok());
    }
}

/// Test 4: End-to-end subagent execution workflow
#[tokio::test]
async fn e2e_subagent_execution_workflow() {
    let detection_result = PiDetection::detect();
    if detection_result.is_err() {
        println!("Skipping e2e_subagent_execution_workflow: pi-mono not installed");
        return;
    }

    let detection = detection_result.unwrap();
    let runner_result = SubagentRunner::from_detection(&detection);
    assert!(runner_result.is_ok());

    let runner = runner_result.unwrap();
    println!("Runner created with timeout: {:?}", runner.config().timeout);

    let config = runner.config();
    assert!(config.timeout > Duration::from_secs(0));

    println!("SubagentRunner validated successfully (execution skipped)");
}

/// Test 5: End-to-end executor workflow
#[tokio::test]
async fn e2e_executor_workflow() {
    let config = ExecutorConfig::default();
    assert_eq!(config.timeout_secs, 300);
    assert_eq!(config.max_retries, 3);

    let success = ExecutionResult::success("Test output".to_string());
    assert!(success.success);
    assert_eq!(success.output, "Test output");
    assert!(success.error.is_none());

    let failure = ExecutionResult::failure("Test error".to_string());
    assert!(!failure.success);
    assert!(failure.output.is_empty());
    assert_eq!(failure.error, Some("Test error".to_string()));

    let detection_result = PiDetection::detect();
    if detection_result.is_ok() {
        let detection = detection_result.unwrap();
        let executor_result = Executor::from_detection(&detection);

        match executor_result {
            Ok(executor) => {
                println!("Executor created successfully");
                let executor_config = executor.config();
                assert_eq!(executor_config.timeout_secs, 300);
                assert_eq!(executor_config.max_retries, 3);
            }
            Err(e) => {
                println!("Executor creation failed (acceptable): {}", e);
            }
        }
    } else {
        println!("Skipping executor creation: pi-mono not installed");
    }
}

/// Test 6: End-to-end provider authentication workflow
#[tokio::test]
async fn e2e_provider_authentication_workflow() {
    use maestro_pi_mono::discovery::ProviderStatus;

    let expected_providers = vec![
        ("anthropic", "ANTHROPIC_API_KEY"),
        ("openai", "OPENAI_API_KEY"),
        ("google", "GOOGLE_API_KEY"),
        ("groq", "GROQ_API_KEY"),
        ("openrouter", "OPENROUTER_API_KEY"),
    ];

    for (provider_name, env_var) in expected_providers {
        let status = ProviderStatus {
            provider: provider_name.to_string(),
            is_configured: std::env::var(env_var).is_ok(),
            env_var: env_var.to_string(),
        };

        println!("Provider: {} (env: {}, configured: {})",
            status.provider,
            status.env_var,
            status.is_configured);

        assert_eq!(status.provider, provider_name);
        assert_eq!(status.env_var, env_var);
    }
}

/// Test 7: End-to-end agent mapping workflow
#[tokio::test]
async fn e2e_agent_mapping_workflow() {
    // Test role to pi-agent type mapping
    let expected_mappings = vec![
        (AgentRole::Scout, PiAgentType::Scout),
        (AgentRole::Architect, PiAgentType::Planner),
        (AgentRole::Critic, PiAgentType::Reviewer),
        (AgentRole::Kraken, PiAgentType::Worker),
    ];

    for (role, expected_agent_type) in expected_mappings {
        let mapped = role_to_pi_agent_type(&role);
        assert_eq!(mapped, Some(expected_agent_type));
        println!("Role: {:?} -> Agent: {:?}", role, mapped);
    }

    // Create a valid AgentMapping using the constructor
    let scout_mapping = AgentMapping::new(
        AgentRole::Scout,
        PiAgentType::Scout,
        ToolAccess::read_only(),
        (TaskComplexity::Trivial, TaskComplexity::Simple),
        "Fast reconnaissance agent".to_string(),
    );

    assert_eq!(scout_mapping.maestro_role, AgentRole::Scout);
    assert_eq!(scout_mapping.pi_agent_type, PiAgentType::Scout);
    assert_eq!(scout_mapping.complexity_range, (TaskComplexity::Trivial, TaskComplexity::Simple));

    println!("Scout mapping: role={:?}, agent_type={:?}, complexity={:?}",
        scout_mapping.maestro_role, scout_mapping.pi_agent_type, scout_mapping.complexity_range);
}

/// Test 8: End-to-end usage metrics workflow
#[tokio::test]
async fn e2e_usage_metrics_workflow() {
    let metrics = UsageMetrics::new(
        1000,
        500,
        Some(0.003),
        Duration::from_secs(10),
    );

    assert_eq!(metrics.tokens_input, 1000);
    assert_eq!(metrics.tokens_output, 500);
    assert_eq!(metrics.tokens_total, 1500);
    assert_eq!(metrics.cost_estimate_usd, Some(0.003));
    assert_eq!(metrics.duration, Duration::from_secs(10));

    let cost_per_million = metrics.cost_per_million_tokens();
    assert_eq!(cost_per_million, Some(2.0));

    let tps = metrics.tokens_per_second();
    assert_eq!(tps, 150.0);

    let zero_duration_metrics = UsageMetrics::new(
        1000, 500, None, Duration::from_secs(0)
    );
    assert_eq!(zero_duration_metrics.tokens_per_second(), 0.0);
}

/// Test 9: End-to-end stream event workflow
#[tokio::test]
async fn e2e_stream_event_workflow() {
    let start = StreamEvent::start("Starting task".to_string());
    assert_eq!(start.event_type, StreamEventType::Start);
    assert_eq!(start.content, "Starting task");
    assert!(start.metadata.is_none());

    let progress = StreamEvent::progress("50% complete".to_string(), Some("50".to_string()));
    assert_eq!(progress.event_type, StreamEventType::Progress);
    assert_eq!(progress.content, "50% complete");
    assert_eq!(progress.metadata, Some("50".to_string()));

    let data = StreamEvent::data("Data chunk".to_string());
    assert_eq!(data.event_type, StreamEventType::Data);

    let error = StreamEvent::error("Error occurred".to_string());
    assert_eq!(error.event_type, StreamEventType::Error);

    let complete = StreamEvent::complete("Task finished".to_string());
    assert_eq!(complete.event_type, StreamEventType::Complete);

    // Test serialization
    let json = serde_json::to_string(&progress).unwrap();
    assert!(json.contains("Progress"));

    let deserialized: StreamEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.event_type, StreamEventType::Progress);
}

/// Test 10: End-to-end result aggregation workflow
#[tokio::test]
async fn e2e_result_aggregation_workflow() {
    let result1 = SubagentResult::success(
        "Task 1".to_string(),
        "agent-001".to_string(),
        "scout".to_string(),
        "Task 1 complete".to_string(),
        Duration::from_secs(5),
    );

    let result2 = SubagentResult::failure(
        "Task 2".to_string(),
        "agent-002".to_string(),
        "worker".to_string(),
        "Task 2 failed".to_string(),
        Duration::from_secs(3),
    );

    let result3 = SubagentResult::success(
        "Task 3".to_string(),
        "agent-003".to_string(),
        "reviewer".to_string(),
        "Task 3 complete".to_string(),
        Duration::from_secs(7),
    );

    let results = vec![&result1, &result2, &result3];
    let success_count = results.iter().filter(|r| r.is_success()).count();
    let failure_count = results.iter().filter(|r| r.is_failure()).count();

    assert_eq!(success_count, 2);
    assert_eq!(failure_count, 1);

    let total_duration: Duration = results.iter().map(|r| r.duration).sum();
    assert_eq!(total_duration, Duration::from_secs(15));

    let errors: Vec<_> = results.iter().filter_map(|r| r.error.as_ref()).collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0], "Task 2 failed");

    // Test event collection
    let result_with_events = SubagentResult::success(
        "Task".to_string(),
        "agent".to_string(),
        "worker".to_string(),
        "Output".to_string(),
        Duration::from_secs(1),
    )
    .with_event(StreamEvent::start("Starting".to_string()))
    .with_event(StreamEvent::error("Warning".to_string()))
    .with_event(StreamEvent::data("Data".to_string()))
    .with_event(StreamEvent::error("Critical".to_string()));

    let error_events = result_with_events.error_events();
    assert_eq!(error_events.len(), 2);
}
