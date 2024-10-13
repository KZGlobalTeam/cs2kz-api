//! Bunnyhop statistics.
//!
//! Currently these are only tracked because they're interesting.
//! In the future, they may be used to detect cheaters as well.

use serde::{Deserialize, Deserializer, Serialize};

/// Statistics about bhop distribution during e.g. an in-game session.
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
	sqlx::FromRow,
	utoipa::ToSchema,
)]
pub struct BhopStats
{
	/// The total count.
	///
	/// This includes `perfs` and `perfect_perfs`.
	#[sqlx(rename = "bhops_total")]
	pub total: u32,

	/// The "perf" count.
	///
	/// A "perf" is whatever the current [mode] considers to be a "perfect"
	/// bhop. This does **not** mean "tick-perfect"! That's what
	/// `perfect_perfs` is for.
	///
	/// [mode]: cs2kz::Mode
	#[sqlx(rename = "bhops_perfs")]
	pub perfs: u32,

	/// The tick-perfect-bhop count.
	#[sqlx(rename = "bhops_perfect_perfs")]
	pub perfect_perfs: u32,
}

impl BhopStats
{
	/// Deserializes [`BhopStats`] and makes sure the contained values are
	/// logically correct (e.g. `perfs <= total` should always hold).
	pub fn deserialize_checked<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let stats = Self::deserialize(deserializer)?;

		if stats.perfs > stats.total {
			return Err(serde::de::Error::custom("`perfs` cannot be greater than `total`"));
		}

		if stats.perfect_perfs > stats.total {
			return Err(serde::de::Error::custom("`perfect_perfs` cannot be greater than `total`"));
		}

		if stats.perfect_perfs > stats.perfs {
			return Err(serde::de::Error::custom("`perfect_perfs` cannot be greater than `perfs`"));
		}

		Ok(stats)
	}
}

#[cfg(test)]
impl fake::Dummy<fake::Faker> for BhopStats
{
	fn dummy_with_rng<R: rand::Rng + ?Sized>(_: &fake::Faker, rng: &mut R) -> Self
	{
		let total = rng.r#gen::<u16>();
		let perfs = rng.gen_range(0..=total);
		let perfect_perfs = rng.gen_range(0..=perfs);

		Self { total, perfs, perfect_perfs }
	}
}
