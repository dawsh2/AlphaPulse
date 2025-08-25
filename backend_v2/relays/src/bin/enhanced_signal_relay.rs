//! # Enhanced Signal Relay with Health Checks and Service Discovery
//!
//! Demonstrates the integration of:
//! - Service Discovery for dynamic socket path resolution
//! - Health Check endpoints for deployment automation
//! - Performance monitoring with metrics collection
//! - Environment-aware configuration
//!
//! This is the next-generation relay architecture that replaces hardcoded paths
//! and enables zero-downtime deployments with automatic failover.

use alphapulse_health_check::{HealthCheckServer, MetricsCollector, ServiceHealth};
use alphapulse_relays::SignalRelay;
use alphapulse_service_discovery::{Environment, ServiceDiscovery, ServiceType};
use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "enhanced_signal_relay")]
#[command(about = "Enhanced Signal Relay with health checks and service discovery")]
struct Args {
    /// Override environment detection
    #[arg(long)]
    environment: Option<String>,

    /// Override socket path
    #[arg(long)]
    socket_path: Option<String>,

    /// Health check port
    #[arg(long, default_value = "8002")]
    health_port: u16,

    /// Enable performance monitoring
    #[arg(long, default_value = "true")]
    enable_monitoring: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("🚀 Starting Enhanced Signal Relay");
    info!("====================================");

    // Initialize service discovery
    let discovery = if let Some(env_str) = &args.environment {
        let env = match env_str.as_str() {
            "development" => Environment::Development,
            "staging" => Environment::Staging,
            "production" => Environment::Production,
            "testing" => Environment::Testing,
            "docker" => Environment::Docker,
            _ => {
                warn!("Unknown environment '{}', using auto-detection", env_str);
                Environment::detect()
            }
        };
        ServiceDiscovery::for_environment(env).await?
    } else {
        ServiceDiscovery::new().await?
    };

    info!("🔍 Environment: {:?}", discovery.environment());
    info!("📁 Socket directory: {}", discovery.socket_dir());
    info!("📋 Log directory: {}", discovery.log_dir());

    // Resolve socket path using service discovery
    let socket_path = if let Some(custom_path) = args.socket_path {
        custom_path
    } else {
        let endpoint = discovery.resolve_service(&ServiceType::SignalRelay).await?;
        endpoint.socket_path
    };

    info!("🔌 Signal Relay socket: {}", socket_path);

    // Initialize health monitoring
    let mut service_health = ServiceHealth::new("signal_relay");
    service_health.set_socket_path(&socket_path);
    service_health.set_health_port(args.health_port);
    service_health.add_detail("environment", &format!("{:?}", discovery.environment()));
    service_health.add_detail("socket_path", &socket_path);
    service_health.add_detail("monitoring_enabled", &args.enable_monitoring.to_string());

    let health_server = HealthCheckServer::new(service_health, args.health_port);
    let health_handle = Arc::new(health_server);

    // Initialize metrics collector
    let metrics_collector = Arc::new(MetricsCollector::new());

    // Start health check server
    let health_server_handle = {
        let health_server = Arc::clone(&health_handle);
        tokio::spawn(async move {
            if let Err(e) = health_server.start().await {
                error!("Health check server failed: {}", e);
            }
        })
    };

    info!(
        "🏥 Health check server started on port {}",
        args.health_port
    );
    info!("   Endpoints: http://127.0.0.1:{}/health", args.health_port);
    info!("              http://127.0.0.1:{}/ready", args.health_port);
    info!(
        "              http://127.0.0.1:{}/metrics",
        args.health_port
    );
    info!("              http://127.0.0.1:{}/status", args.health_port);

    // Create relay configuration with dynamic socket path
    let mut relay_config = alphapulse_relays::SignalRelayConfig::default();
    // Configure based on environment
    match discovery.environment() {
        Environment::Production => {
            // Production settings for high throughput
            relay_config.max_consumers = 1000;
            relay_config.channel_buffer_size = 10000;
            relay_config.cleanup_interval_ms = 30000;
        }
        Environment::Staging => {
            // Staging settings
            relay_config.max_consumers = 100;
            relay_config.channel_buffer_size = 1000;
            relay_config.cleanup_interval_ms = 10000;
        }
        _ => {
            // Development/Testing settings
            relay_config.max_consumers = 50;
            relay_config.channel_buffer_size = 500;
            relay_config.cleanup_interval_ms = 5000;
        }
    }

    // Initialize the relay
    let mut relay = SignalRelay::new(socket_path, relay_config);

    // Update health status to starting
    health_handle
        .update_health(|health| {
            health.status = alphapulse_health_check::HealthStatus::Starting;
            health.add_detail("relay_status", "initializing");
        })
        .await;

    // Start performance monitoring task
    let monitoring_handle = if args.enable_monitoring {
        let health_server = Arc::clone(&health_handle);
        let metrics = Arc::clone(&metrics_collector);

        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));

            loop {
                interval.tick().await;

                let performance_metrics = metrics.get_metrics();

                health_server
                    .update_health(|health| {
                        health.update_metrics(performance_metrics.clone());
                        health.add_detail("last_metrics_update", &chrono::Utc::now().to_rfc3339());
                    })
                    .await;

                // Log performance metrics periodically
                if performance_metrics.total_messages > 0 {
                    info!(
                        "📊 Performance: {:.0} msg/s, {} active connections, {} total messages",
                        performance_metrics.messages_per_second,
                        performance_metrics.active_connections,
                        performance_metrics.total_messages
                    );
                }
            }
        }))
    } else {
        None
    };

    // Start the relay
    info!("🔄 Starting Signal Relay...");
    let relay_handle = tokio::spawn(async move {
        if let Err(e) = relay.start().await {
            error!("Signal Relay failed: {}", e);
            return Err(e);
        }
        Ok(())
    });

    // Update health status to healthy once relay is running
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    health_handle
        .update_health(|health| {
            health.status = alphapulse_health_check::HealthStatus::Healthy;
            health.add_detail("relay_status", "running");
            health.add_detail("startup_completed", &chrono::Utc::now().to_rfc3339());
        })
        .await;

    info!("✅ Enhanced Signal Relay is running!");
    info!("🎯 Features enabled:");
    info!("   • Dynamic service discovery");
    info!("   • Health check endpoints");
    info!("   • Performance monitoring");
    info!("   • Environment-aware configuration");
    info!("   • Zero-downtime deployment support");

    // Set up graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
    };

    tokio::select! {
        result = relay_handle => {
            match result {
                Ok(Ok(())) => info!("Signal Relay completed successfully"),
                Ok(Err(e)) => error!("Signal Relay error: {}", e),
                Err(e) => error!("Signal Relay task error: {}", e),
            }
        }
        _ = shutdown_signal => {
            info!("🛑 Received shutdown signal, gracefully stopping...");

            // Update health status
            health_handle.update_health(|health| {
                health.status = alphapulse_health_check::HealthStatus::Unhealthy;
                health.add_detail("relay_status", "shutting_down");
            }).await;

            // Allow health checks to report shutdown status
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    // Clean up tasks
    if let Some(monitoring_task) = monitoring_handle {
        monitoring_task.abort();
    }
    health_server_handle.abort();

    info!("🏁 Enhanced Signal Relay shutdown complete");

    Ok(())
}
