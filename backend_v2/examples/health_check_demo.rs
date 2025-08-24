//! Health Check System Demo
//! Shows how to add health monitoring to any service

use alphapulse_health_check::{HealthCheckServer, ServiceHealth, MetricsCollector};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏥 AlphaPulse Health Check Demo");
    println!("===============================");

    // 1. Create service health tracker
    let mut health = ServiceHealth::new("demo_service");
    health.set_socket_path("/tmp/alphapulse/demo.sock");
    health.set_health_port(8001);
    health.add_detail("version", "1.0.0");
    health.add_detail("environment", "demo");

    // 2. Start health check server
    let health_server = HealthCheckServer::new(health, 8001);
    println!("🚀 Starting health check server on port 8001...");
    
    let server_handle = {
        let server = Arc::new(health_server);
        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            server_clone.start().await
        })
    };

    // 3. Simulate service activity with metrics
    let metrics = MetricsCollector::new();
    
    println!("📊 Simulating service activity...");
    println!();
    println!("🔍 Available endpoints:");
    println!("  http://127.0.0.1:8001/health   - Liveness check");
    println!("  http://127.0.0.1:8001/ready    - Readiness check");
    println!("  http://127.0.0.1:8001/metrics  - Performance metrics");
    println!("  http://127.0.0.1:8001/status   - Detailed status");
    println!();

    // Simulate processing messages
    for i in 1..=20 {
        metrics.increment_messages();
        metrics.set_active_connections(i % 5 + 1);
        
        let current_metrics = metrics.get_metrics();
        println!("📈 Processed {} messages ({:.1} msg/s, {} connections)", 
                 current_metrics.total_messages, 
                 current_metrics.messages_per_second,
                 current_metrics.active_connections);
        
        sleep(Duration::from_secs(2)).await;
    }

    println!();
    println!("✅ Demo complete! Health endpoints remain active.");
    println!("💡 Try: curl http://127.0.0.1:8001/metrics");
    
    // Keep server running
    server_handle.await??;
    Ok(())
}