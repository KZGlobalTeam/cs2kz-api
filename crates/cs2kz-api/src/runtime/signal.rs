use tokio::signal;

pub async fn shutdown() {
    select! {
        () = sigint() => {},
        () = sigterm() => {},
    }
}

async fn sigint() {
    match signal::ctrl_c().await {
        Ok(()) => warn!("shutting down"),
        Err(error) => error!(%error, "failed to listen for ctrl-c"),
    }
}

#[cfg(unix)]
async fn sigterm() {
    use tokio::signal::unix;

    match unix::signal(unix::SignalKind::terminate()) {
        Ok(mut signal) => match signal.recv().await {
            Some(()) => warn!("shutting down"),
            None => error!("could not listen for more SIGTERM events"),
        },
        Err(error) => error!(%error, "failed to listen for SIGTERM"),
    }
}

#[cfg(not(unix))]
async fn sigterm() {
    std::future::pending().await
}
