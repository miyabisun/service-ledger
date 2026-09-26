mod logging;
mod port;

use std::{error::Error, net::SocketAddr, path::PathBuf};

use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    logging::init();

    let bind_addr = SocketAddr::from(([0, 0, 0, 0], port::from_env()?));
    let db_path = std::env::var_os("DB_PATH")
        .filter(|path| !path.is_empty())
        .map_or_else(|| PathBuf::from("data/service-ledger.db"), PathBuf::from);
    let store = service_ledger::Store::open(&db_path)
        .map_err(|_| format!("cannot open the SQLite database at {}", db_path.display()))?;
    let listener = TcpListener::bind(bind_addr).await?;
    info!(%bind_addr, db_path = %db_path.display(), "server listening");

    axum::serve(listener, service_ledger::app(store))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("shutdown signal received");
}
