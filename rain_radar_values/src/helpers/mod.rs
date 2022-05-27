mod cross_product;
pub(crate) use cross_product::*;

#[cfg(any(test, feature = "local_file_analysis"))]
pub mod local_file_analysis;
