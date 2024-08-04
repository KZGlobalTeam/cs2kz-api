//! API services.
//!
//! These contain the core business logic.
//!
//! If a service directly maps to an HTTP route, it will have an `http` module
//! containing the handlers and an `Into<axum::Router>` implementation.

/* TODO:
 * - AntiCheat service
 *    - perhaps implement this as middleware?
 */

pub mod steam;
pub use steam::SteamService;

pub mod auth;
pub use auth::AuthService;

pub mod health;
pub use health::HealthService;

pub mod players;
pub use players::PlayerService;

pub mod maps;
pub use maps::MapService;

pub mod servers;
pub use servers::ServerService;

pub mod records;
pub use records::RecordService;

pub mod jumpstats;
pub use jumpstats::JumpstatService;

pub mod bans;
pub use bans::BanService;

pub mod admins;
pub use admins::AdminService;

pub mod plugin;
pub use plugin::PluginService;
