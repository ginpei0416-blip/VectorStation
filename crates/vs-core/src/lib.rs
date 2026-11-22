//! VectorStation の中核。書類(ドキュメントモデル)と、それを変更するコマンド、Undo/Redo。
//! 画面にもOSにも依存しない(ブラウザ版にそのまま載せるため)。
//! 設計: docs/design/data-model.md

pub mod attributes;
pub mod command;
pub mod error;
pub mod history;
pub mod id;
pub mod model;
pub mod patch;
pub mod shape;

pub use attributes::{Attr, Attributes, Color, Fill, LineType, Resolved};
pub use command::Command;
pub use error::CoreError;
pub use history::History;
pub use id::Id;
pub use model::{Class, DesignLayer, Document, LayerProps, Object, Visibility};
pub use shape::{Point, Shape, Vertex, VertexKind};
