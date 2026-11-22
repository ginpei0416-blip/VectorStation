use serde::{Deserialize, Serialize};
use std::fmt;

/// 変わらない識別番号。名前を変えても参照が切れないように、参照はすべてIDで行う。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(pub String);

impl Id {
    /// 新しいIDを作る。`prefix` は種類が人に見分けられるように付ける(cls, lay, obj など)。
    pub fn new(prefix: &str) -> Self {
        Id(format!("{prefix}-{}", uuid::Uuid::new_v4().simple()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id(s.to_string())
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
