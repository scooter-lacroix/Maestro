//! MaestroClaw daemon — supervised long-running autonomous runtime

use crate::config::Config;
use crate::observability::{create_observer, Observer, ObserverEvent, TelemetryCorrelation};
use anyhow::{anyhow, Result};
use chrono::Utc;
use std::future::Future;
use std::path::PathBuf;
#[cfg(feature = "channels")]
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::Duration;

pub async fn run(config: Config) -> Result<()> {
    let observer = create_observer(&config.observability.backend, Some(&config.workspace_dir));
    let runtime_correlation = TelemetryCorrelation::default()
        .with_component("runtime")
        .with_surface("runtime")
        .normalized_with(None, Some("runtime".into()), Some("runtime"));
    let daemon_correlation = TelemetryCorrelation::default()
        .with_component("daemon")
        .with_surface("runtime")
        .normalized_with(None, Some("daemon".into()), Some("runtime"));
    observer.record_correlated_event(
        &ObserverEvent::RuntimeStart {
            workspace_dir: config.workspace_dir.to_string_lossy().to_string(),
        },
        runtime_correlation.clone(),
    );
    observer.record_correlated_event(&ObserverEvent::DaemonStart, daemon_correlation.clone());

    let initial_backoff = config.daemon.initial_backoff_secs.max(1);
    let max_backoff = config.daemon.max_backoff_secs.max(initial_backoff);

    crate::health::mark_component_ok("daemon");
    observer.record_correlated_event(
        &ObserverEvent::ComponentHealth {
            component: "daemon".to_string(),
            healthy: true,
            restart_count: 0,
        },
        daemon_correlation.clone(),
    );

    let mut handles: Vec<JoinHandle<()>> = vec![spawn_state_writer(config.clone())];
    let mut components = Vec::new();

    if config.cron.enabled {
        let sched_config = config.clone();
        handles.push(spawn_supervisor(
            "scheduler",
            initial_backoff,
            max_backoff,
            observer.clone(),
            move || {
                let cfg = sched_config.clone();
                async move { crate::cron::scheduler::run(&cfg).await }
            },
        ));
        components.push("scheduler");
    } else {
        mark_component_idle("scheduler", observer.as_ref());
        tracing::info!("Cron disabled; scheduler not started");
    }

    #[cfg(feature = "gateway")]
    {
        let gw_config = config.clone();
        handles.push(spawn_supervisor(
            "gateway",
            initial_backoff,
            max_backoff,
            observer.clone(),
            move || {
                let cfg = gw_config.clone();
                async move { crate::gateway::run_gateway(cfg).await }
            },
        ));
        components.push("gateway");
    }

    if config.heartbeat.enabled {
        let heartbeat_config = config.clone();
        handles.push(spawn_supervisor(
            "heartbeat",
            initial_backoff,
            max_backoff,
            observer.clone(),
            move || {
                let cfg = heartbeat_config.clone();
                async move {
                    crate::heartbeat::HeartbeatEngine::ensure_heartbeat_file(&cfg.workspace_dir)
                        .await?;
                    crate::heartbeat::HeartbeatEngine::new(&cfg).run().await
                }
            },
        ));
        components.push("heartbeat");
    } else {
        mark_component_idle("heartbeat", observer.as_ref());
    }

    #[cfg(feature = "channels")]
    {
        if let Some(channel_config) = config.channels.telegram.clone() {
            let dispatcher_config = config.clone();
            let workspace_dir = config.workspace_dir.clone();
            handles.push(spawn_supervisor(
                "channel:telegram",
                initial_backoff,
                max_backoff,
                observer.clone(),
                move || {
                    let channel_config = channel_config.clone();
                    let dispatcher_config = dispatcher_config.clone();
                    let workspace_dir = workspace_dir.clone();
                    async move {
                        let channel: Arc<dyn crate::channels::Channel> =
                            Arc::new(crate::channels::telegram::TelegramChannel::new(
                                channel_config.bot_token.clone(),
                                channel_config.allowed_users.clone(),
                            ));
                        let dispatcher = crate::channels::ChannelDispatcher::new(
                            &dispatcher_config,
                            &workspace_dir,
                        );
                        run_channel_runtime("channel:telegram", channel, dispatcher).await
                    }
                },
            ));
            components.push("channel:telegram");
        } else {
            mark_component_idle("channel:telegram", observer.as_ref());
        }

        if let Some(channel_config) = config.channels.discord.clone() {
            let dispatcher_config = config.clone();
            let workspace_dir = config.workspace_dir.clone();
            handles.push(spawn_supervisor(
                "channel:discord",
                initial_backoff,
                max_backoff,
                observer.clone(),
                move || {
                    let channel_config = channel_config.clone();
                    let dispatcher_config = dispatcher_config.clone();
                    let workspace_dir = workspace_dir.clone();
                    async move {
                        let channel: Arc<dyn crate::channels::Channel> =
                            Arc::new(crate::channels::discord::DiscordChannel::new(
                                channel_config.bot_token.clone(),
                                channel_config.guild_id.clone(),
                                channel_config.allowed_users.clone(),
                            ));
                        let dispatcher = crate::channels::ChannelDispatcher::new(
                            &dispatcher_config,
                            &workspace_dir,
                        );
                        run_channel_runtime("channel:discord", channel, dispatcher).await
                    }
                },
            ));
            components.push("channel:discord");
        } else {
            mark_component_idle("channel:discord", observer.as_ref());
        }

        if let Some(channel_config) = config.channels.slack.clone() {
            let dispatcher_config = config.clone();
            let workspace_dir = config.workspace_dir.clone();
            handles.push(spawn_supervisor(
                "channel:slack",
                initial_backoff,
                max_backoff,
                observer.clone(),
                move || {
                    let channel_config = channel_config.clone();
                    let dispatcher_config = dispatcher_config.clone();
                    let workspace_dir = workspace_dir.clone();
                    async move {
                        let channel: Arc<dyn crate::channels::Channel> =
                            Arc::new(crate::channels::slack::SlackChannel::new(
                                channel_config.bot_token.clone(),
                                channel_config.app_token.clone(),
                                channel_config.allowed_users.clone(),
                            ));
                        let dispatcher = crate::channels::ChannelDispatcher::new(
                            &dispatcher_config,
                            &workspace_dir,
                        );
                        run_channel_runtime("channel:slack", channel, dispatcher).await
                    }
                },
            ));
            components.push("channel:slack");
        } else {
            mark_component_idle("channel:slack", observer.as_ref());
        }
    }

    println!("🧠 MaestroClaw daemon started");
    #[cfg(feature = "gateway")]
    println!(
        "   Gateway:  http://{}:{}",
        config.gateway.host, config.gateway.port
    );
    println!("   Components: {}", components.join(", "));
    println!("   Ctrl+C to stop");

    tokio::signal::ctrl_c().await?;
    crate::health::mark_component_error("daemon", "shutdown requested");
    observer.record_correlated_event(&ObserverEvent::DaemonStop, daemon_correlation);
    observer.record_correlated_event(&ObserverEvent::RuntimeStop, runtime_correlation);

    for handle in &handles {
        handle.abort();
    }
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

pub fn state_file_path(config: &Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("daemon_state.json")
}

fn spawn_state_writer(config: Config) -> JoinHandle<()> {
    tokio::spawn(async move {
        let path = state_file_path(&config);
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let flush_secs = config.daemon.state_flush_secs.max(1);
        let mut interval = tokio::time::interval(Duration::from_secs(flush_secs));
        loop {
            interval.tick().await;
            let mut json = crate::health::snapshot_json();
            if let Some(obj) = json.as_object_mut() {
                obj.insert(
                    "written_at".into(),
                    serde_json::json!(Utc::now().to_rfc3339()),
                );
            }
            let data = serde_json::to_vec_pretty(&json).unwrap_or_else(|_| b"{}".to_vec());
            let _ = tokio::fs::write(&path, data).await;
        }
    })
}

fn spawn_supervisor<F, Fut>(
    name: &'static str,
    initial_backoff: u64,
    max_backoff: u64,
    observer: Box<dyn Observer>,
    mut run_fn: F,
) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        let mut backoff = initial_backoff.max(1);
        let mut restart_count = 0u32;
        let correlation = TelemetryCorrelation::default()
            .with_component(name)
            .with_surface("runtime")
            .normalized_with(None, Some(name.to_string()), Some("runtime"));

        loop {
            crate::health::mark_component_ok(name);
            observer.record_correlated_event(
                &ObserverEvent::ComponentHealth {
                    component: name.to_string(),
                    healthy: true,
                    restart_count,
                },
                correlation.clone(),
            );

            match run_fn().await {
                Ok(()) => {
                    let message = format!("component '{name}' exited unexpectedly");
                    crate::health::mark_component_error(name, &message);
                    tracing::warn!("{message}");
                    observer.record_correlated_event(
                        &ObserverEvent::RuntimeError { error: message },
                        correlation.clone(),
                    );
                    observer.record_correlated_event(
                        &ObserverEvent::ComponentHealth {
                            component: name.to_string(),
                            healthy: false,
                            restart_count,
                        },
                        correlation.clone(),
                    );
                    backoff = initial_backoff.max(1);
                }
                Err(error) => {
                    let message = format!("component '{name}' failed: {error}");
                    crate::health::mark_component_error(name, &message);
                    tracing::error!("{message}");
                    observer.record_correlated_event(
                        &ObserverEvent::RuntimeError { error: message },
                        correlation.clone(),
                    );
                    observer.record_correlated_event(
                        &ObserverEvent::ComponentHealth {
                            component: name.to_string(),
                            healthy: false,
                            restart_count,
                        },
                        correlation.clone(),
                    );
                }
            }

            crate::health::bump_component_restart(name);
            restart_count = restart_count.saturating_add(1);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
            backoff = backoff.saturating_mul(2).min(max_backoff);
        }
    })
}

fn mark_component_idle(component: &str, observer: &dyn Observer) {
    crate::health::mark_component_ok(component);
    observer.record_correlated_event(
        &ObserverEvent::ComponentHealth {
            component: component.to_string(),
            healthy: true,
            restart_count: 0,
        },
        TelemetryCorrelation::default()
            .with_component(component)
            .with_surface("runtime")
            .normalized_with(None, Some(component.to_string()), Some("runtime")),
    );
}

#[cfg(feature = "channels")]
async fn run_channel_runtime(
    component: &'static str,
    channel: Arc<dyn crate::channels::Channel>,
    dispatcher: crate::channels::ChannelDispatcher,
) -> Result<()> {
    if !channel.health_check().await {
        crate::health::mark_component_error(component, "channel health check failed");
        return Err(anyhow!("channel health check failed for {component}"));
    }

    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let listen_channel = channel.clone();
    let dispatch_channel = channel;

    tokio::select! {
        result = listen_channel.listen(tx) => result,
        result = dispatcher.run(dispatch_channel, rx) => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn state_file_uses_config_dir() {
        let tmp = TempDir::new().unwrap();
        let config = Config {
            config_path: tmp.path().join("config.toml"),
            ..Config::default()
        };
        let path = state_file_path(&config);
        assert_eq!(path, tmp.path().join("daemon_state.json"));
    }
}
