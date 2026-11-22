use thiserror::Error;

/// 操作できなかった理由。メッセージは作成者が読んでわかる言葉で書く。
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CoreError {
    #[error("クラスが見つかりません: {0}")]
    ClassNotFound(String),
    #[error("デザインレイヤが見つかりません: {0}")]
    LayerNotFound(String),
    #[error("図形が見つかりません: {0}")]
    ObjectNotFound(String),
    #[error("同じIDがすでにあります: {0}")]
    DuplicateId(String),
    #[error("「{0}」クラスは削除できません(Vectorworks と同じく必須のクラスです)")]
    BuiltinClass(String),
    #[error("レイヤ「{0}」はロックされています")]
    LayerLocked(String),
    #[error("図形 {0} はロックされています")]
    ObjectLocked(String),
    #[error("まだ対応していない種類の図形は、この操作ができません: {0}")]
    UnsupportedShape(String),
    #[error("縮尺は1以上の数にしてください: {0}")]
    InvalidScale(f64),
    #[error("名前が空です")]
    EmptyName,
    #[error("デザインレイヤは最低1枚必要です")]
    LastLayer,
    #[error("値が正しくありません: {0}")]
    InvalidValue(String),
    #[error("取り消せる操作がありません")]
    NothingToUndo,
    #[error("やり直せる操作がありません")]
    NothingToRedo,
}
