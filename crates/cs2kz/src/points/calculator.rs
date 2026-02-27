use std::io;
use std::time::Duration;

use futures_util::TryFutureExt as _;
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::Config;
use crate::maps::courses::Tier;
use crate::points::DistributionParameters;
use crate::python::Python;

type Message = (Request, oneshot::Sender<Response>);

#[derive(Debug)]
pub struct PointsCalculator {
    python: Python<Request, Response>,
    chan: (mpsc::Sender<Message>, mpsc::Receiver<Message>),
}

#[derive(Debug, Clone)]
pub struct PointsCalculatorHandle {
    chan: mpsc::Sender<Message>,
}

#[derive(Debug, Display, Error)]
pub enum Error {
    #[display("python error")]
    Python(io::Error),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Request {
    pub time: f64,
    pub nub_data: LeaderboardData,
    pub pro_data: Option<LeaderboardData>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Response {
    pub nub_fraction: f64,
    pub pro_fraction: Option<f64>,
}

#[derive(Debug, Display, Error)]
#[display("failed to calculate points ({_variant})")]
pub enum CalculatePointsError {
    #[display("calculator unavailable")]
    CalculatorUnavailable,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LeaderboardData {
    pub dist_params: Option<DistributionParameters>,
    pub tier: Tier,
    pub leaderboard_size: u64,
    #[serde(rename = "wr")]
    pub top_time: f64,
}

impl PointsCalculator {
    pub async fn new(config: &Config) -> io::Result<Option<Self>> {
        let Some(script_path) = config.points.calc_run_path.as_deref() else {
            tracing::warn!(
                "no `points.calc-run-path` configured; points calculator will be disabled"
            );
            return Ok(None);
        };

        let python = Python::new(script_path.to_owned(), config.database.url.clone()).await?;
        let chan = mpsc::channel(128);

        Ok(Some(Self { python, chan }))
    }

    pub fn handle(&self) -> PointsCalculatorHandle {
        PointsCalculatorHandle { chan: self.chan.0.clone() }
    }

    #[tracing::instrument(skip_all)]
    pub async fn run(mut self, cancellation_token: CancellationToken) -> Result<(), Error> {
        loop {
            select! {
                () = cancellation_token.cancelled() => {
                    tracing::debug!("cancelled");
                    break Ok(());
                },

                Some((request, response_tx)) = self.chan.1.recv() => {
                    loop {
                        match self.python.send_request(&request).await {
                            Ok(response) => {
                                _ = response_tx.send(response);
                                break;
                            },
                            Err(err) => {
                                tracing::error!(%err, "failed to execute python request");
                                self.python.reset().map_err(Error::Python).await?;
                                sleep(Duration::from_secs(1)).await;
                            },
                        }
                    }
                },
            };
        }
    }
}

impl PointsCalculatorHandle {
    pub async fn calculate(&self, request: Request) -> Result<Response, CalculatePointsError> {
        let (response_tx, response_rx) = oneshot::channel::<Response>();

        if let Err(_send_err) = self.chan.send((request, response_tx)).await {
            return Err(CalculatePointsError::CalculatorUnavailable);
        }

        match response_rx.await {
            Ok(response) => Ok(response),
            Err(_recv_err) => Err(CalculatePointsError::CalculatorUnavailable),
        }
    }
}
