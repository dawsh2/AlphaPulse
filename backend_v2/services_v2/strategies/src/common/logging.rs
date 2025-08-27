//! Common logging configuration for strategy services

use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

/// Initialize standardized logging for strategy services
pub fn init_strategy_logging(service_name: &str) -> Result<()> {
    // Create filter with service-specific defaults
    let filter = EnvFilter::from_default_env()
        .add_directive("info".parse()?)
        .add_directive(format!("{}=info", service_name).parse()?)
        .add_directive("alphapulse_strategies=info".parse()?)
        .add_directive("alphapulse_state_market=info".parse()?)
        .add_directive("alphapulse_adapter_service=info".parse()?)
        .add_directive("torq_network=warn".parse()?)
        .add_directive("ethers=warn".parse()?);

    // Configure formatter with consistent format
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_span_events(FmtSpan::NONE)
        .with_ansi(atty::is(atty::Stream::Stderr))
        .compact()
        .try_init()?;

    Ok(())
}

/// Initialize logging for testing with debug level
pub fn init_test_logging() -> Result<()> {
    let filter = EnvFilter::from_default_env()
        .add_directive("debug".parse()?)
        .add_directive("alphapulse_strategies=debug".parse()?)
        .add_directive("torq_network=info".parse()?)
        .add_directive("ethers=warn".parse()?);

    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .try_init();

    Ok(())
}