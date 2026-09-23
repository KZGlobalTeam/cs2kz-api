use std::cmp;
use std::time::Duration;

use crate::time::DurationExt;

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::Type)]
#[serde(rename_all = "kebab-case")]
#[sqlx(rename_all = "kebab-case")]
pub enum BanReason {
    AHK,
    StrafeHack,

    BhopMacro,
    BhopHack,

    Hyperscroll,

    /// Invalid client cvar values
    InjectedCvar,

    /// Impossible input values
    InjectedInput,

    Nulls,

    SubtickSpam,

    Desubtick,
    // Other, // ???
}

impl BanReason {
    /// Calculates the default ban duration for ban of this reason.
    ///
    /// `total_ban_duration` is the total duration the player has been banned for in the past.
    pub fn duration(&self, total_ban_duration: Duration) -> Duration {
        let mut duration = match self {
            // Self::Macro => Duration::week() * 2,
            // Self::AutoBhop => Duration::month(),
            // Self::AutoStrafe => Duration::month() * 2,
            Self::AHK => Duration::week(),
            Self::StrafeHack => Duration::year(),

            Self::BhopMacro => Duration::month(),
            Self::BhopHack => Duration::year(),

            Self::Hyperscroll => Duration::week(),

            Self::InjectedCvar => Duration::year() * 5,
            Self::InjectedInput => Duration::year() * 5,

            Self::Nulls => Duration::week(),

            Self::SubtickSpam => Duration::week(),
            Self::Desubtick => Duration::week(),
        };

        if !total_ban_duration.is_zero() {
            duration = (duration + total_ban_duration) * 2;
        }

        cmp::min(duration, Duration::year() * 5)
    }
}
