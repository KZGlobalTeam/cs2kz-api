use std::cmp;
use std::time::Duration;

use crate::time::DurationExt;

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::Type)]
#[serde(rename_all = "kebab-case")]
#[sqlx(rename_all = "kebab-case")]
pub enum BanReason {
    Macro,
    AutoBhop,
    AutoStrafe,
}

impl BanReason {
    /// Calculates the default ban duration for ban of this reason.
    ///
    /// `total_ban_duration` is the total duration the player has been banned for in the past.
    pub fn duration(&self, total_ban_duration: Duration) -> Duration {
        let mut duration = match self {
            Self::Macro => Duration::week() * 2,
            Self::AutoBhop => Duration::month(),
            Self::AutoStrafe => Duration::month() * 2,
        };

        if !total_ban_duration.is_zero() {
            duration = (duration + total_ban_duration) * 2;
        }

        cmp::min(duration, Duration::year())
    }
}
