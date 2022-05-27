#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![feature(allocator_api)]

mod rain_radar_values;
pub use crate::rain_radar_values::*;

mod dwd_rain_radar_values;
pub use dwd_rain_radar_values::*;

pub mod compressed_rain_radar_values;
pub use compressed_rain_radar_values::*;

mod helpers;
pub(crate) use helpers::*;

// if local_file_analysis is enabled, make it public so that bins can use it too
#[cfg(feature = "local_file_analysis")]
pub use helpers::local_file_analysis;
