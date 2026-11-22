//! 独自形式(.vws)の保存・読込と、古い版からの移行。
//! 方針: 古い版のファイルは必ず新しい版で開ける。知らない項目は消さずに書き戻す。

use serde_json::{json, Value};
use thiserror::Error;
use vs_core::model::{Builtin, Document};

/// 今の形式の版。上げるときは migrate に変換を足し、tests/fixtures に見本を残す。
pub const CURRENT_FORMAT_VERSION: u64 = 1;
pub const EXTENSION: &str = "vws";

#[derive(Debug, Error)]
pub enum IoError {
    #[error("ファイルの中身が読めません(JSON として壊れています): {0}")]
    Json(#[from] serde_json::Error),
    #[error("VectorStation のファイルではありません(formatVersion がありません)")]
    NotVectorStation,
    #[error("このファイルは新しい版(形式 {found})の VectorStation で作られています。今の版(形式 {current})を更新してください")]
    TooNew { found: u64, current: u64 },
    #[error("ファイルの内容に問題があります: {0}")]
    Invalid(String),
}

/// 書類をファイルの文字列にする(人とAIが読めるよう整形した JSON)。
pub fn save_to_string(doc: &Document) -> Result<String, IoError> {
    let v = json!({
        "formatVersion": CURRENT_FORMAT_VERSION,
        "app": "VectorStation",
        "document": serde_json::to_value(doc)?,
    });
    Ok(serde_json::to_string_pretty(&v)? + "\n")
}

/// ファイルの文字列から書類を読む。古い版なら今の版に変換してから読む。
pub fn load_from_str(s: &str) -> Result<Document, IoError> {
    let v: Value = serde_json::from_str(s)?;
    let version = v
        .get("formatVersion")
        .and_then(Value::as_u64)
        .ok_or(IoError::NotVectorStation)?;
    if version > CURRENT_FORMAT_VERSION {
        return Err(IoError::TooNew {
            found: version,
            current: CURRENT_FORMAT_VERSION,
        });
    }
    let v = migrate(v, version)?;
    let doc_v = v
        .get("document")
        .cloned()
        .ok_or_else(|| IoError::Invalid("document がありません".into()))?;
    let doc: Document = serde_json::from_value(doc_v)?;
    validate(&doc)?;
    Ok(doc)
}

/// 古い版から今の版へ、1段ずつ変換する。
/// 例: 形式2を作るときは `1 => v = v1_to_v2(v)?` を足す。
// 形式2ができて変換が入るまで、到達しない行がある警告を止める
#[allow(unreachable_code, clippy::never_loop, clippy::match_single_binding)]
fn migrate(mut v: Value, from: u64) -> Result<Value, IoError> {
    for ver in from..CURRENT_FORMAT_VERSION {
        v = match ver {
            // 形式1が最初の版なので、まだ変換はない。例: 1 => v1_to_v2(v)?,
            _ => {
                return Err(IoError::Invalid(format!(
                    "形式 {ver} からの変換が見つかりません"
                )))
            }
        };
    }
    v["formatVersion"] = json!(CURRENT_FORMAT_VERSION);
    Ok(v)
}

/// 読み込んだ書類の最低限の約束事を確認する。
fn validate(doc: &Document) -> Result<(), IoError> {
    for b in [Builtin::None, Builtin::Dimension] {
        if !doc.classes.iter().any(|c| c.builtin == Some(b)) {
            return Err(IoError::Invalid(format!("必須クラス({b:?})がありません")));
        }
    }
    if doc.design_layers.is_empty() {
        return Err(IoError::Invalid("デザインレイヤが1枚もありません".into()));
    }
    let mut seen = std::collections::HashSet::new();
    let ids = doc
        .classes
        .iter()
        .map(|c| &c.id)
        .chain(doc.design_layers.iter().map(|l| &l.props.id))
        .chain(
            doc.design_layers
                .iter()
                .flat_map(|l| l.objects.iter().map(|o| &o.id)),
        );
    for id in ids {
        if !seen.insert(id) {
            return Err(IoError::Invalid(format!("IDが重複しています: {id}")));
        }
    }
    for o in doc.design_layers.iter().flat_map(|l| &l.objects) {
        if doc.class(&o.class_id).is_none() {
            return Err(IoError::Invalid(format!(
                "図形 {} のクラス {} がありません",
                o.id, o.class_id
            )));
        }
    }
    Ok(())
}
