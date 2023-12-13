//! This module holds middleware functions for authentication.

mod gameserver;
pub use gameserver::verify_gameserver;

mod map_approval;
pub use map_approval::verify_map_admin;

mod admins_only;
pub use admins_only::verify_admin;
