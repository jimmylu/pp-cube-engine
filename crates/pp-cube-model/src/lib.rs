//! Contract types, model loading and baseline validation.

mod load;
mod meta;
mod model;
mod query;
mod reference;

pub use load::{load_json, load_query, load_yaml, LoadError, Position};
pub use model::{
    Cube, Dimension, DimensionType, Join, Measure, MeasureFilter, MeasureType, Model, Relationship,
    Segment,
};
pub use query::{Condition, Direction, Filter, Operator, Order, Query};
pub use reference::{
    locate_references, parse_references, Located, Reference, ReferenceError, Target,
};
