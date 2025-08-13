// Prometheus metrics handler
use axum::{extract::State, response::Response, http::StatusCode};
use crate::state::AppState;

pub async fn prometheus_metrics(State(state): State<AppState>) -> Result<Response<String>, StatusCode> {
    // Record the metrics request
    state.metrics.record_http_request("/metrics", 200);
    
    // Get metrics from the Prometheus registry
    let metrics_output = metrics_exporter_prometheus::PrometheusBuilder::new()
        .build_recorder()
        .handle()
        .render();
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .body(metrics_output)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(response)
}