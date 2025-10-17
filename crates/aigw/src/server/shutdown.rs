use async_trait::async_trait;
use pingora_core::server::{RunArgs, ShutdownSignal, ShutdownSignalWatch};

#[cfg(unix)]
use tokio::sync::broadcast::Sender;
pub struct MyUnixShutdownSignalWatch {
    tx: std::sync::Arc<Sender<()>>,
}

#[cfg(unix)]
#[async_trait]
impl ShutdownSignalWatch for MyUnixShutdownSignalWatch {
    async fn recv(&self) -> ShutdownSignal {
        let mut graceful_upgrade_signal =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::quit()).unwrap();
        let mut graceful_terminate_signal =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        let mut fast_shutdown_signal =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = graceful_upgrade_signal.recv() => {
                let _= self.tx.send(());
                ShutdownSignal::GracefulUpgrade
            },
            _ = graceful_terminate_signal.recv() => {
                let _= self.tx.send(());
                ShutdownSignal::GracefulTerminate
            },
            _ = fast_shutdown_signal.recv() => {
                let _= self.tx.send(());
                ShutdownSignal::FastShutdown
            },
        }
    }
}

#[cfg(unix)]
pub(crate) fn run_args(tx: std::sync::Arc<Sender<()>>) -> RunArgs {

    RunArgs {
        shutdown_signal: Box::new(MyUnixShutdownSignalWatch { tx }),
    }
}

#[cfg(windows)]
pub(crate) fn run_args(tx: std::sync::Arc<Sender<()>>) -> RunArgs {
    RunArgs::default()
}
