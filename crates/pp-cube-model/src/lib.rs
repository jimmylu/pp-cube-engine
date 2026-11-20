//! Contract types, model loading and baseline validation.

mod load;
mod meta;
mod model;
mod reference;

pub use load::{load_json, load_yaml, LoadError, Position};
pub use model::{
    Cube, Dimension, DimensionType, Join, Measure, MeasureFilter, MeasureType, Model, Relationship,
    Segment,
};
pub use reference::{parse_references, Reference, ReferenceError, Target};
