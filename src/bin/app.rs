use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use adapter::database::connection_database_with;
use adapter::redis::RedisClient;
use anyhow::{Context, Result};
use api::route::auth;
use api::route::{book::build_book_routers, health::build_health_check_routers};
use axum::Router;
use registry::AppRegistry;
use shared::config::AppConfig;
use shared::env::{Environment, which};
use tokio::{
    net::TcpListener,
    signal::{
        self,
        unix::{SignalKind, signal},
    },
};
use tower_http::LatencyUnit;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger()?;
    bootstrap().await
}

async fn bootstrap() -> Result<()> {
    let app_config = AppConfig::new()?;
    let pool = connection_database_with(&app_config.database);
    let kv = Arc::new(RedisClient::new(&app_config.redis)?);
    let registry = AppRegistry::new(pool, kv, app_config);

    let app = Router::new()
        .merge(build_health_check_routers())
        .merge(build_book_routers())
        .merge(auth::routes())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .with_state(registry);

    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8080);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("Listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Unexpected error happened in server")
        .inspect_err(|e| {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Unexpected error"
            )
        })
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal(SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _=ctrl_c=>{},
        _=terminate=>{},
    }

    println!("Start graceful shutdown");
}

fn init_logger() -> Result<()> {
    let log_level = match which() {
        Environment::Development => "debug",
        Environment::Production => "info",
    };

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| log_level.into());
    let subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_target(false);

    tracing_subscriber::registry()
        .with(subscriber)
        .with(env_filter)
        .try_init()?;

    Ok(())
}
