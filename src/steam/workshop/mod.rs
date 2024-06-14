//! Steam Workshop functionality.

use crate::make_id;

mod map_info;
pub use map_info::fetch_map_name;

mod map_file;
pub use map_file::MapFile;

make_id!(WorkshopID as u32);
