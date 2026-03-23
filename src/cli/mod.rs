//! CLI command handlers for Redguard Preservation

pub mod convert;
mod filetype;
pub mod read;
pub mod scan;
pub mod utils;

pub use convert::handle_convert_command;
pub use read::handle_read_command;
pub use scan::handle_scan_command;
