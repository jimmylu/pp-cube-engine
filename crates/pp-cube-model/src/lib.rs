//! Contract types, model loading and baseline validation.

mod load;
mod model;

pub use load::{load_json, load_yaml, LoadError};
pub use model::{Cube, Dimension, DimensionType, Measure, MeasureFilter, MeasureType, Model};
