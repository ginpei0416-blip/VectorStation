//! コマンド: 書類を変更する唯一の入口。画面操作も CLI も同じコマンドを使う。
//! JSON で `{"cmd": "addObject", ...}` のように書ける。

use crate::attributes::{Attributes, ClassAttributes};
use crate::error::CoreError;
use crate::id::Id;
use crate::model::{Class, DesignLayer, Document, Extra, LayerProps, Object, Visibility};
use crate::patch::Patch;
use crate::shape::Shape;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 図形の形の指定(`{"type": "line", "geometry": {...}}`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeSpec {
    #[serde(rename = "type")]
    pub kind: String,
    pub geometry: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Command {
    // --- クラス ---
    AddClass {
        #[serde(default)]
        id: Option<Id>,
        name: String,
        #[serde(default)]
        attributes: Option<ClassAttributes>,
        #[serde(default)]
        use_at_creation: Option<bool>,
    },
    SetClass {
        id: Id,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        visibility: Option<Visibility>,
        #[serde(default)]
        attributes: Option<ClassAttributes>,
        #[serde(default)]
        use_at_creation: Option<bool>,
    },
    /// クラスを削除する。そのクラスの図形は「一般」クラス(英語版の None)に移る
    DeleteClass {
        id: Id,
    },
    /// 並び順を変える(オーガナイザのドラッグ&ドロップ)
    MoveClass {
        id: Id,
        to: usize,
    },

    // --- デザインレイヤ ---
    AddDesignLayer {
        #[serde(default)]
        id: Option<Id>,
        name: String,
        #[serde(default = "default_scale")]
        scale: f64,
    },
    SetDesignLayer {
        id: Id,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        scale: Option<f64>,
        #[serde(default)]
        visibility: Option<Visibility>,
        #[serde(default)]
        locked: Option<bool>,
        #[serde(default)]
        elevation: Option<f64>,
    },
    DeleteDesignLayer {
        id: Id,
    },
    MoveDesignLayer {
        id: Id,
        to: usize,
    },

    /// アクティブなレイヤ・クラスを切り替える
    SetActive {
        #[serde(default)]
        layer: Option<Id>,
        #[serde(default)]
        class: Option<Id>,
    },

    // --- 図形 ---
    /// 図形を描く。レイヤ・クラスを省略するとアクティブなものを使う
    AddObject {
        #[serde(default)]
        id: Option<Id>,
        #[serde(default)]
        layer: Option<Id>,
        #[serde(default)]
        class: Option<Id>,
        shape: ShapeSpec,
        #[serde(default)]
        attributes: Option<Attributes>,
    },
    DeleteObjects {
        ids: Vec<Id>,
    },
    /// 数値指定の移動(⌘M)
    MoveObjects {
        ids: Vec<Id>,
        dx: f64,
        dy: f64,
    },
    /// 複製(⌘D)。ずらす量を指定できる。複製はレイヤの一番手前に置く
    DuplicateObjects {
        ids: Vec<Id>,
        #[serde(default)]
        dx: f64,
        #[serde(default)]
        dy: f64,
    },
    /// 属性を変える。指定した項目だけが変わる
    SetObjectAttributes {
        ids: Vec<Id>,
        attributes: Attributes,
    },
    SetObjectClass {
        ids: Vec<Id>,
        class: Id,
    },
    SetObjectLocked {
        ids: Vec<Id>,
        locked: bool,
    },
}

fn default_scale() -> f64 {
    50.0
}

/// 変更の途中経過。失敗したら、それまでの変更を元に戻す。
pub(crate) struct Tx<'a> {
    pub doc: &'a mut Document,
    pub patches: Vec<Patch>,
}

impl<'a> Tx<'a> {
    pub fn new(doc: &'a mut Document) -> Self {
        Tx {
            doc,
            patches: vec![],
        }
    }
    fn push(&mut self, p: Patch) -> Result<(), CoreError> {
        p.apply(self.doc)?;
        self.patches.push(p);
        Ok(())
    }
    pub fn rollback(self) {
        for p in self.patches.iter().rev() {
            let _ = p.inverse().apply(self.doc);
        }
    }
}

fn check_name(name: &str) -> Result<(), CoreError> {
    if name.trim().is_empty() {
        Err(CoreError::EmptyName)
    } else {
        Ok(())
    }
}

fn check_scale(s: f64) -> Result<(), CoreError> {
    if s.is_finite() && s >= 1.0 {
        Ok(())
    } else {
        Err(CoreError::InvalidScale(s))
    }
}

fn check_opacity(a: &Attributes) -> Result<(), CoreError> {
    use crate::attributes::Attr;
    for v in [&a.pen_opacity, &a.fill_opacity].into_iter().flatten() {
        if let Attr::Value { value } = v {
            if *value > 100 {
                return Err(CoreError::InvalidValue(format!(
                    "不透明度は0〜100です: {value}"
                )));
            }
        }
    }
    if let Some(Attr::Value { value }) = &a.line_weight {
        if !(value.is_finite() && *value >= 0.0) {
            return Err(CoreError::InvalidValue(format!("線の太さ: {value}")));
        }
    }
    Ok(())
}

fn new_id(doc: &Document, given: &Option<Id>, prefix: &str) -> Result<Id, CoreError> {
    match given {
        Some(id) if doc.id_in_use(id) => Err(CoreError::DuplicateId(id.to_string())),
        Some(id) => Ok(id.clone()),
        None => Ok(Id::new(prefix)),
    }
}

/// 編集できる図形か(図形自体とレイヤのロックを確認)。
fn editable(doc: &Document, id: &Id) -> Result<(usize, usize), CoreError> {
    let (li, oi) = doc
        .find_object(id)
        .ok_or_else(|| CoreError::ObjectNotFound(id.to_string()))?;
    let layer = &doc.design_layers[li];
    if layer.props.locked {
        return Err(CoreError::LayerLocked(layer.props.name.clone()));
    }
    if layer.objects[oi].locked {
        return Err(CoreError::ObjectLocked(id.to_string()));
    }
    Ok((li, oi))
}

impl Command {
    /// コマンドを実行する。成功したら変更の記録と、新しく作られたIDを返す。
    /// 失敗したら書類は元のまま。
    pub fn execute(&self, doc: &mut Document) -> Result<(Vec<Patch>, Vec<Id>), CoreError> {
        let mut tx = Tx::new(doc);
        match self.run(&mut tx) {
            Ok(created) => Ok((tx.patches, created)),
            Err(e) => {
                tx.rollback();
                Err(e)
            }
        }
    }

    fn run(&self, tx: &mut Tx) -> Result<Vec<Id>, CoreError> {
        use Command::*;
        let mut created = vec![];
        match self {
            AddClass {
                id,
                name,
                attributes,
                use_at_creation,
            } => {
                check_name(name)?;
                let id = new_id(tx.doc, id, "cls")?;
                let mut c = Class::new(id.clone(), name.clone());
                c.attributes = attributes.clone().unwrap_or_default();
                c.use_at_creation = use_at_creation.unwrap_or(false);
                let index = tx.doc.classes.len();
                tx.push(Patch::ClassInsert { index, class: c })?;
                created.push(id);
            }
            SetClass {
                id,
                name,
                visibility,
                attributes,
                use_at_creation,
            } => {
                let before = tx
                    .doc
                    .class(id)
                    .ok_or_else(|| CoreError::ClassNotFound(id.to_string()))?
                    .clone();
                let mut after = before.clone();
                if let Some(n) = name {
                    check_name(n)?;
                    after.name = n.clone();
                }
                if let Some(v) = visibility {
                    after.visibility = *v;
                }
                if let Some(a) = attributes {
                    after.attributes = a.clone();
                }
                if let Some(u) = use_at_creation {
                    after.use_at_creation = *u;
                }
                tx.push(Patch::ClassReplace { before, after })?;
            }
            DeleteClass { id } => {
                let class = tx
                    .doc
                    .class(id)
                    .ok_or_else(|| CoreError::ClassNotFound(id.to_string()))?
                    .clone();
                if class.builtin.is_some() {
                    return Err(CoreError::BuiltinClass(class.name));
                }
                let none = tx.doc.none_class_id();
                let moved: Vec<Object> = tx
                    .doc
                    .design_layers
                    .iter()
                    .flat_map(|l| l.objects.iter())
                    .filter(|o| &o.class_id == id)
                    .cloned()
                    .collect();
                for before in moved {
                    let mut after = before.clone();
                    after.class_id = none.clone();
                    tx.push(Patch::ObjectReplace { before, after })?;
                }
                if tx.doc.state.active_class.as_ref() == Some(id) {
                    let before = tx.doc.state.clone();
                    let mut after = before.clone();
                    after.active_class = Some(none.clone());
                    tx.push(Patch::StateReplace { before, after })?;
                }
                let index = tx.doc.class_index(id).unwrap();
                tx.push(Patch::ClassRemove { index, class })?;
            }
            MoveClass { id, to } => {
                let from = tx
                    .doc
                    .class_index(id)
                    .ok_or_else(|| CoreError::ClassNotFound(id.to_string()))?;
                let to = (*to).min(tx.doc.classes.len() - 1);
                if from != to {
                    tx.push(Patch::ClassMove { from, to })?;
                }
            }
            AddDesignLayer { id, name, scale } => {
                check_name(name)?;
                check_scale(*scale)?;
                let id = new_id(tx.doc, id, "lay")?;
                let layer = DesignLayer {
                    props: LayerProps {
                        id: id.clone(),
                        name: name.clone(),
                        scale: *scale,
                        visibility: Visibility::Visible,
                        locked: false,
                        elevation: 0.0,
                        story_id: None,
                        extra: Extra::new(),
                    },
                    objects: vec![],
                };
                let index = tx.doc.design_layers.len();
                tx.push(Patch::LayerInsert { index, layer })?;
                // Vectorworks と同じく、新しいレイヤをアクティブにする
                let before = tx.doc.state.clone();
                let mut after = before.clone();
                after.active_layer = Some(id.clone());
                tx.push(Patch::StateReplace { before, after })?;
                created.push(id);
            }
            SetDesignLayer {
                id,
                name,
                scale,
                visibility,
                locked,
                elevation,
            } => {
                let before = tx
                    .doc
                    .layer(id)
                    .ok_or_else(|| CoreError::LayerNotFound(id.to_string()))?
                    .props
                    .clone();
                let mut after = before.clone();
                if let Some(n) = name {
                    check_name(n)?;
                    after.name = n.clone();
                }
                if let Some(s) = scale {
                    check_scale(*s)?;
                    after.scale = *s;
                }
                if let Some(v) = visibility {
                    after.visibility = *v;
                }
                if let Some(l) = locked {
                    after.locked = *l;
                }
                if let Some(e) = elevation {
                    after.elevation = *e;
                }
                tx.push(Patch::LayerPropsReplace { before, after })?;
            }
            DeleteDesignLayer { id } => {
                let index = tx
                    .doc
                    .layer_index(id)
                    .ok_or_else(|| CoreError::LayerNotFound(id.to_string()))?;
                if tx.doc.design_layers.len() == 1 {
                    return Err(CoreError::LastLayer);
                }
                let layer = tx.doc.design_layers[index].clone();
                if tx.doc.state.active_layer.as_ref() == Some(id) {
                    let other = tx
                        .doc
                        .design_layers
                        .iter()
                        .find(|l| &l.props.id != id)
                        .unwrap()
                        .props
                        .id
                        .clone();
                    let before = tx.doc.state.clone();
                    let mut after = before.clone();
                    after.active_layer = Some(other);
                    tx.push(Patch::StateReplace { before, after })?;
                }
                tx.push(Patch::LayerRemove { index, layer })?;
            }
            MoveDesignLayer { id, to } => {
                let from = tx
                    .doc
                    .layer_index(id)
                    .ok_or_else(|| CoreError::LayerNotFound(id.to_string()))?;
                let to = (*to).min(tx.doc.design_layers.len() - 1);
                if from != to {
                    tx.push(Patch::LayerMove { from, to })?;
                }
            }
            SetActive { layer, class } => {
                let before = tx.doc.state.clone();
                let mut after = before.clone();
                if let Some(l) = layer {
                    tx.doc
                        .layer(l)
                        .ok_or_else(|| CoreError::LayerNotFound(l.to_string()))?;
                    after.active_layer = Some(l.clone());
                }
                if let Some(c) = class {
                    tx.doc
                        .class(c)
                        .ok_or_else(|| CoreError::ClassNotFound(c.to_string()))?;
                    after.active_class = Some(c.clone());
                }
                tx.push(Patch::StateReplace { before, after })?;
            }
            AddObject {
                id,
                layer,
                class,
                shape,
                attributes,
            } => {
                let layer_id = layer
                    .clone()
                    .or_else(|| tx.doc.state.active_layer.clone())
                    .unwrap_or_else(|| tx.doc.design_layers[0].props.id.clone());
                let l = tx
                    .doc
                    .layer(&layer_id)
                    .ok_or_else(|| CoreError::LayerNotFound(layer_id.to_string()))?;
                if l.props.locked {
                    return Err(CoreError::LayerLocked(l.props.name.clone()));
                }
                let index = l.objects.len();
                let class_id = class
                    .clone()
                    .or_else(|| tx.doc.state.active_class.clone())
                    .unwrap_or_else(|| tx.doc.none_class_id());
                let cls = tx
                    .doc
                    .class(&class_id)
                    .ok_or_else(|| CoreError::ClassNotFound(class_id.to_string()))?;
                let shape = Shape::from_parts(&shape.kind, shape.geometry.clone())
                    .map_err(CoreError::InvalidValue)?;
                if let Shape::Unknown { kind, .. } = &shape {
                    return Err(CoreError::UnsupportedShape(kind.clone()));
                }
                // 「作成時にクラスの属性を使う」なら、全項目をクラスに従わせてから、指定分を上書き
                let base = if cls.use_at_creation {
                    Attributes::all_by_class()
                } else {
                    Attributes::default()
                };
                let attrs = match attributes {
                    Some(a) => {
                        check_opacity(a)?;
                        base.merged_with(a)
                    }
                    None => base,
                };
                let id = new_id(tx.doc, id, "obj")?;
                let object = Object {
                    id: id.clone(),
                    shape,
                    class_id,
                    attributes: attrs,
                    locked: false,
                    extra: Extra::new(),
                };
                tx.push(Patch::ObjectInsert {
                    layer: layer_id,
                    index,
                    object,
                })?;
                created.push(id);
            }
            DeleteObjects { ids } => {
                for id in ids {
                    let (li, oi) = editable(tx.doc, id)?;
                    let layer = tx.doc.design_layers[li].props.id.clone();
                    let object = tx.doc.design_layers[li].objects[oi].clone();
                    tx.push(Patch::ObjectRemove {
                        layer,
                        index: oi,
                        object,
                    })?;
                }
            }
            MoveObjects { ids, dx, dy } => {
                for id in ids {
                    let (li, oi) = editable(tx.doc, id)?;
                    let before = tx.doc.design_layers[li].objects[oi].clone();
                    let shape = before.shape.translated(*dx, *dy).ok_or_else(|| {
                        CoreError::UnsupportedShape(before.shape.kind().to_string())
                    })?;
                    let mut after = before.clone();
                    after.shape = shape;
                    tx.push(Patch::ObjectReplace { before, after })?;
                }
            }
            DuplicateObjects { ids, dx, dy } => {
                for id in ids {
                    let (li, oi) = tx
                        .doc
                        .find_object(id)
                        .ok_or_else(|| CoreError::ObjectNotFound(id.to_string()))?;
                    let layer = &tx.doc.design_layers[li];
                    if layer.props.locked {
                        return Err(CoreError::LayerLocked(layer.props.name.clone()));
                    }
                    let src = layer.objects[oi].clone();
                    let shape = src
                        .shape
                        .translated(*dx, *dy)
                        .ok_or_else(|| CoreError::UnsupportedShape(src.shape.kind().to_string()))?;
                    let new = Id::new("obj");
                    let object = Object {
                        id: new.clone(),
                        shape,
                        locked: false,
                        ..src
                    };
                    let layer_id = layer.props.id.clone();
                    let index = layer.objects.len();
                    tx.push(Patch::ObjectInsert {
                        layer: layer_id,
                        index,
                        object,
                    })?;
                    created.push(new);
                }
            }
            SetObjectAttributes { ids, attributes } => {
                check_opacity(attributes)?;
                for id in ids {
                    let (li, oi) = editable(tx.doc, id)?;
                    let before = tx.doc.design_layers[li].objects[oi].clone();
                    let mut after = before.clone();
                    after.attributes = before.attributes.merged_with(attributes);
                    tx.push(Patch::ObjectReplace { before, after })?;
                }
            }
            SetObjectClass { ids, class } => {
                tx.doc
                    .class(class)
                    .ok_or_else(|| CoreError::ClassNotFound(class.to_string()))?;
                for id in ids {
                    let (li, oi) = editable(tx.doc, id)?;
                    let before = tx.doc.design_layers[li].objects[oi].clone();
                    let mut after = before.clone();
                    after.class_id = class.clone();
                    tx.push(Patch::ObjectReplace { before, after })?;
                }
            }
            SetObjectLocked { ids, locked } => {
                for id in ids {
                    let (li, oi) = tx
                        .doc
                        .find_object(id)
                        .ok_or_else(|| CoreError::ObjectNotFound(id.to_string()))?;
                    let layer = &tx.doc.design_layers[li];
                    if layer.props.locked {
                        return Err(CoreError::LayerLocked(layer.props.name.clone()));
                    }
                    let before = layer.objects[oi].clone();
                    let mut after = before.clone();
                    after.locked = *locked;
                    tx.push(Patch::ObjectReplace { before, after })?;
                }
            }
        }
        Ok(created)
    }
}
