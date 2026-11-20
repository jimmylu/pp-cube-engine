//! Contract type for a query: Cube's query format, narrowed to what the
//! kernel compiles. A field it does not compile yet does not deserialize.

use serde::{Deserialize, Serialize};

/// Members are written `cube.member`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    #[serde(default)]
    pub measures: Vec<String>,
    #[serde(default)]
    pub dimensions: Vec<String>,
}
