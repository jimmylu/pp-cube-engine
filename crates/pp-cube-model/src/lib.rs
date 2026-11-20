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
pub use query::{
    Condition, DateRange, DateValue, Direction, Filter, Granularity, Operator, Order, Query,
    TimeDimension,
};
pub use reference::{
    locate_references, parse_references, Located, Reference, ReferenceError, Target,
};

mod validation;
pub use validation::{validate_json, validate_model, validate_yaml, ValidationIssue};
