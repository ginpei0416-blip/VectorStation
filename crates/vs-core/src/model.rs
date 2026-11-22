//! 書類(ドキュメント)の構造。docs/design/data-model.md の 1〜6 章に対応。
//! 未対応の項目は `extra` にそのまま保持し、保存時に書き戻す(消さない)。

use crate::attributes::{Attributes, ClassAttributes};
use crate::id::Id;
use crate::shape::Shape;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub type Extra = Map<String, Value>;

/// 表示状態(Vectorworks と同じ3種類)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Visibility {
    #[default]
    Visible,
    Grayed,
    Hidden,
}

/// 必須クラスの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Builtin {
    /// 「なし」
    None,
    /// 「寸法」
    Dimension,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Class {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub visibility: Visibility,
    #[serde(default)]
    pub attributes: ClassAttributes,
    /// 作成時にクラスの属性を使う
    #[serde(default)]
    pub use_at_creation: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin: Option<Builtin>,
    #[serde(flatten)]
    pub extra: Extra,
}

impl Class {
    pub fn new(id: Id, name: impl Into<String>) -> Self {
        Class {
            id,
            name: name.into(),
            visibility: Visibility::Visible,
            attributes: ClassAttributes::default(),
            use_at_creation: false,
            builtin: None,
            extra: Extra::new(),
        }
    }
}

/// デザインレイヤの設定(図形以外)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerProps {
    pub id: Id,
    pub name: String,
    /// 縮尺の分母(1:50 なら 50)
    pub scale: f64,
    #[serde(default)]
    pub visibility: Visibility,
    #[serde(default)]
    pub locked: bool,
    /// 高さ(mm)。3D・ストーリ用の予約
    #[serde(default)]
    pub elevation: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story_id: Option<Id>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignLayer {
    #[serde(flatten)]
    pub props: LayerProps,
    /// 図形の並び(後ろほど手前)
    #[serde(default)]
    pub objects: Vec<Object>,
}

/// 図形。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawObject", into = "RawObject")]
pub struct Object {
    pub id: Id,
    pub shape: Shape,
    pub class_id: Id,
    pub attributes: Attributes,
    pub locked: bool,
    pub extra: Extra,
}

/// ファイル上の図形の形(type と geometry を分けて持つ)。
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawObject {
    id: Id,
    #[serde(rename = "type")]
    kind: String,
    class_id: Id,
    #[serde(default)]
    geometry: Value,
    #[serde(default)]
    attributes: Attributes,
    #[serde(default, skip_serializing_if = "is_false")]
    locked: bool,
    #[serde(flatten)]
    extra: Extra,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl TryFrom<RawObject> for Object {
    type Error = String;
    fn try_from(r: RawObject) -> Result<Self, String> {
        Ok(Object {
            id: r.id,
            shape: Shape::from_parts(&r.kind, r.geometry)?,
            class_id: r.class_id,
            attributes: r.attributes,
            locked: r.locked,
            extra: r.extra,
        })
    }
}

impl From<Object> for RawObject {
    fn from(o: Object) -> Self {
        let (kind, geometry) = o.shape.to_parts();
        RawObject {
            id: o.id,
            kind,
            class_id: o.class_id,
            geometry,
            attributes: o.attributes,
            locked: o.locked,
            extra: o.extra,
        }
    }
}

/// 書類全体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    #[serde(default)]
    pub settings: Settings,
    pub classes: Vec<Class>,
    pub design_layers: Vec<DesignLayer>,
    /// シートレイヤ(Phase 6)。今は中身をそのまま保持する
    #[serde(default)]
    pub sheet_layers: Vec<Value>,
    /// ストーリ(予約)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stories: Vec<Value>,
    /// リソース(線種、ハッチング、模様、タイル…)。今は中身をそのまま保持する
    #[serde(default)]
    pub resources: Extra,
    /// 別ファイルの参照(予約)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<Value>,
    #[serde(default)]
    pub state: State,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub units: String,
    #[serde(flatten)]
    pub extra: Extra,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            units: "mm".into(),
            extra: Extra::new(),
        }
    }
}

/// 作業状態(アクティブなレイヤ・クラス)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_layer: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_class: Option<Id>,
    #[serde(flatten)]
    pub extra: Extra,
}

pub const NONE_CLASS_ID: &str = "cls-none";
pub const DIMENSION_CLASS_ID: &str = "cls-dimension";

impl Document {
    /// 新規書類。Vectorworks と同じく「なし」「寸法」クラスと、デザインレイヤ1枚(1:50)を持つ。
    pub fn new() -> Self {
        let mut none = Class::new(Id::from(NONE_CLASS_ID), "なし");
        none.builtin = Some(Builtin::None);
        let mut dim = Class::new(Id::from(DIMENSION_CLASS_ID), "寸法");
        dim.builtin = Some(Builtin::Dimension);
        let layer_id = Id::from("lay-1");
        Document {
            settings: Settings::default(),
            classes: vec![none, dim],
            design_layers: vec![DesignLayer {
                props: LayerProps {
                    id: layer_id.clone(),
                    name: "デザインレイヤ-1".into(),
                    scale: 50.0,
                    visibility: Visibility::Visible,
                    locked: false,
                    elevation: 0.0,
                    story_id: None,
                    extra: Extra::new(),
                },
                objects: vec![],
            }],
            sheet_layers: vec![],
            stories: vec![],
            resources: Extra::new(),
            references: vec![],
            state: State {
                active_layer: Some(layer_id),
                active_class: Some(Id::from(NONE_CLASS_ID)),
                extra: Extra::new(),
            },
            extra: Extra::new(),
        }
    }

    pub fn none_class_id(&self) -> Id {
        self.classes
            .iter()
            .find(|c| c.builtin == Some(Builtin::None))
            .map(|c| c.id.clone())
            .unwrap_or_else(|| Id::from(NONE_CLASS_ID))
    }

    pub fn class(&self, id: &Id) -> Option<&Class> {
        self.classes.iter().find(|c| &c.id == id)
    }
    pub fn class_index(&self, id: &Id) -> Option<usize> {
        self.classes.iter().position(|c| &c.id == id)
    }
    pub fn layer(&self, id: &Id) -> Option<&DesignLayer> {
        self.design_layers.iter().find(|l| &l.props.id == id)
    }
    pub fn layer_index(&self, id: &Id) -> Option<usize> {
        self.design_layers.iter().position(|l| &l.props.id == id)
    }

    /// 図形を探す。(レイヤ番号, 図形番号)を返す。
    pub fn find_object(&self, id: &Id) -> Option<(usize, usize)> {
        for (li, l) in self.design_layers.iter().enumerate() {
            if let Some(oi) = l.objects.iter().position(|o| &o.id == id) {
                return Some((li, oi));
            }
        }
        None
    }
    pub fn object(&self, id: &Id) -> Option<&Object> {
        self.find_object(id)
            .map(|(l, o)| &self.design_layers[l].objects[o])
    }

    /// 図形の最終的な属性(クラスに従う項目を解決した値)。
    pub fn resolved_attributes(&self, id: &Id) -> Option<crate::Resolved> {
        let o = self.object(id)?;
        let class_attrs = self
            .class(&o.class_id)
            .map(|c| c.attributes.clone())
            .unwrap_or_default();
        Some(o.attributes.resolve(&class_attrs))
    }

    /// すべてのIDが使われていないか(新しいIDの重複確認用)。
    pub fn id_in_use(&self, id: &Id) -> bool {
        self.class(id).is_some() || self.layer(id).is_some() || self.find_object(id).is_some()
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}
