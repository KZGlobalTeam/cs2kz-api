//! Helper types for interacting with Steam's Workshop.

use crate::id::make_id;

mod map_info;

#[doc(inline)]
pub use map_info::fetch_map_name;

mod map_file;

#[doc(inline)]
pub use map_file::MapFile;

/// URL for fetching map information from Steam's API.
const API_URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

make_id!(WorkshopID as u32);
