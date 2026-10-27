use prm::{app, AppState};
use std::{net::Ipv4Addr, path::PathBuf};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(std::env::var("PRM_DATA_DIR").unwrap_or_else(|_| "data".into()));
    let port = std::env::var("PRM_PORT")
        .unwrap_or_else(|_| "4317".into())
        .parse::<u16>()?;
    let state = AppState::open(
        &directory,
        std::env::var("PRM_AGENT_TOKEN")
            .ok()
            .filter(|token| !token.is_empty()),
    )?;
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
    let timeout_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let _ = timeout_state.purge_expired_sessions();
        }
    });
    println!("PRM is available at http://127.0.0.1:{port}");
    axum::serve(listener, app(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
