use serde::{Deserialize, Serialize};

/// Unix timestamps of the last change to each part of the library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastUpdate {
    pub favorite: Option<i64>,
    pub playlist: Option<i64>,
}

#[derive(Deserialize)]
pub struct LastUpdateResponse {
    pub last_update: LastUpdate,
}
