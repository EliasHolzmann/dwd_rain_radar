#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![feature(allocator_api)]

mod rain_radar_values;
pub use crate::rain_radar_values::*;

mod dwd_rain_radar_values;
pub use dwd_rain_radar_values::*;

pub mod compressed_rain_radar_values;
pub use compressed_rain_radar_values::*;

pub mod cross_product;
use cross_product::*;
