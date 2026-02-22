use std::sync::Arc;
use std::time::Duration;
use std::{fmt, io};

use tokio::task;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tokio_util::time::FutureExt;

use crate::config::Config;
use crate::database::{
    self,
    Database,
    DatabaseConnectionOptions,
    EstablishDatabaseConnectionError,
};
use crate::points;

mod inner {
    use super::*;

    #[derive(Debug)]
    pub(super) struct Context {
        pub(super) config: Config,
        pub(super) database: Database,
        pub(super) shutdown_token: CancellationToken,
        pub(super) tasks: TaskTracker,
        pub(super) points_calculator: Option<points::calculator::PointsCalculatorHandle>,
        pub(super) points_daemon: points::daemon::PointsDaemonHandle,
    }
}

/// The API's global state.
#[derive(Clone)]
pub struct Context(Arc<inner::Context>);

#[derive(Debug, Display, Error, From)]
pub enum InitializeContextError {
    #[display("{_0}")]
    EstablishDatabaseConnection(EstablishDatabaseConnectionError),

    #[display("failed to run database migrations: {_0}")]
    RunDatabaseMigrations(sqlx::migrate::MigrateError),

    #[display("failed to initialize points calculator: {_0}")]
    InitializePointsCalculator(io::Error),
}

impl Context {
    /// Initializes a new [`Context`].
    pub async fn new(config: Config) -> Result<Self, InitializeContextError> {
        Self::with_shutdown_token(config, CancellationToken::new()).await
    }

    /// Initializes a new [`Context`] with the given cancellation token.
    ///
    /// The token will be cancelled by [`Context::shutdown()`] and is given to tasks spawned by the
    /// returned [`Context`].
    #[tracing::instrument(level = "debug", skip(shutdown_token), err)]
    pub async fn with_shutdown_token(
        config: Config,
        shutdown_token: CancellationToken,
    ) -> Result<Self, InitializeContextError> {
        let database = Database::connect(DatabaseConnectionOptions {
            url: &config.database.url,
            min_connections: config.database.min_connections,
            max_connections: config.database.max_connections,
        })
        .await?;

        database::MIGRATIONS.run(database.as_ref()).await?;

        let tasks = TaskTracker::new();
        let points_calculator = points::calculator::PointsCalculator::new(&config)
            .await?
            .map(|calc| {
                let handle = calc.handle();
                let cancellation_token = shutdown_token.child_token();
                let task = tasks.track_future(calc.run(cancellation_token));

                task::Builder::new()
                    .name("cs2kz::points_calculator")
                    .spawn(task)
                    .expect("failed to spawn tokio task");

                handle
            });
        let points_daemon = points::daemon::PointsDaemonHandle::new();

        Ok(Self(Arc::new(inner::Context {
            config,
            database,
            shutdown_token,
            tasks,
            points_calculator,
            points_daemon,
        })))
    }

    pub fn config(&self) -> &Config {
        &self.0.config
    }

    pub(crate) fn database(&self) -> &Database {
        &self.0.database
    }

    pub fn points_calculator(&self) -> Option<&points::calculator::PointsCalculatorHandle> {
        self.0.points_calculator.as_ref()
    }

    pub fn points_daemon(&self) -> &points::daemon::PointsDaemonHandle {
        &self.0.points_daemon
    }

    /// Executes an `async` closure in the context of a database transaction.
    ///
    /// If the closure returns <code>[Ok](())</code>, the transaction will be committed.
    /// If the closure returns <code>[Err](_)</code> or panics, the transaction will be rolled
    /// back.
    pub(crate) fn database_transaction<F, T, E>(&self, f: F) -> impl Future<Output = Result<T, E>>
    where
        F: AsyncFnOnce(&mut database::Connection) -> Result<T, E>,
        E: From<database::Error>,
    {
        self.database().in_transaction(f)
    }

    /// Tracks the future produced by `make_future`.
    ///
    /// `make_future` is given a [`CancellationToken`] the produced future can use to detect when
    /// the server is shutting down. When this happens, all tracked futures will be given some
    /// amount of time to perform cleanup.
    pub fn track_future<T>(
        self,
        make_future: impl AsyncFnOnce(Context, CancellationToken) -> T,
    ) -> impl Future<Output = T> {
        let tasks = self.0.tasks.clone();
        let cancellation_token = self.0.shutdown_token.child_token();

        tasks.track_future(make_future(self, cancellation_token))
    }

    /// Tracks the future produced by `make_future` and spawns it as a tokio task.
    ///
    /// See [`Context::track_future()`] for details on tracked tasks.
    pub fn spawn<F, Fut>(&self, name: &str, make_future: F) -> task::JoinHandle<Fut::Output>
    where
        F: FnOnce(CancellationToken) -> Fut,
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let cancellation_token = self.0.shutdown_token.child_token();
        let task = self.0.tasks.track_future(make_future(cancellation_token));

        task::Builder::new()
            .name(name)
            .spawn(task)
            .expect("failed to spawn tokio task")
    }

    /// Initiates cleanup.
    ///
    /// All tasks spawned by this [`Context`] will be notified and are given a few seconds to
    /// exit. Open database connections are closed gracefully.
    #[tracing::instrument(level = "debug")]
    pub async fn cleanup(self) {
        if !self.0.tasks.is_empty() {
            self.shutdown_tasks(Duration::from_secs(10)).await;
        }

        self.close_database(Duration::from_secs(5)).await;
    }

    #[tracing::instrument(level = "debug")]
    async fn shutdown_tasks(&self, timeout: Duration) {
        self.0.tasks.close();
        self.0.shutdown_token.cancel();

        if let Err(_) = self.0.tasks.wait().timeout(timeout).await {
            warn!(?timeout, "tasks did not shutdown within timeout");
        }
    }

    #[tracing::instrument(level = "debug")]
    async fn close_database(&self, timeout: Duration) {
        if let Err(_) = self.database().cleanup().timeout(timeout).await {
            warn!(?timeout, "failed to cleanup database connections within timeout");
        }
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        <inner::Context as fmt::Debug>::fmt(&*self.0, fmt)
    }
}
