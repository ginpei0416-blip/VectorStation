//! 書類への最小単位の変更。すべての操作はこの組み合わせで表し、
//! 逆向きの変更(inverse)を作れるので Undo/Redo ができる。

use crate::error::CoreError;
use crate::id::Id;
use crate::model::{Class, DesignLayer, Document, LayerProps, Object, State};

#[derive(Debug, Clone, PartialEq)]
pub enum Patch {
    ObjectInsert {
        layer: Id,
        index: usize,
        object: Object,
    },
    ObjectRemove {
        layer: Id,
        index: usize,
        object: Object,
    },
    ObjectReplace {
        before: Object,
        after: Object,
    },
    ClassInsert {
        index: usize,
        class: Class,
    },
    ClassRemove {
        index: usize,
        class: Class,
    },
    ClassReplace {
        before: Class,
        after: Class,
    },
    ClassMove {
        from: usize,
        to: usize,
    },
    LayerInsert {
        index: usize,
        layer: DesignLayer,
    },
    LayerRemove {
        index: usize,
        layer: DesignLayer,
    },
    LayerPropsReplace {
        before: LayerProps,
        after: LayerProps,
    },
    LayerMove {
        from: usize,
        to: usize,
    },
    StateReplace {
        before: State,
        after: State,
    },
}

fn move_item<T>(v: &mut Vec<T>, from: usize, to: usize) {
    let item = v.remove(from);
    v.insert(to, item);
}

impl Patch {
    pub fn inverse(&self) -> Patch {
        use Patch::*;
        match self.clone() {
            ObjectInsert {
                layer,
                index,
                object,
            } => ObjectRemove {
                layer,
                index,
                object,
            },
            ObjectRemove {
                layer,
                index,
                object,
            } => ObjectInsert {
                layer,
                index,
                object,
            },
            ObjectReplace { before, after } => ObjectReplace {
                before: after,
                after: before,
            },
            ClassInsert { index, class } => ClassRemove { index, class },
            ClassRemove { index, class } => ClassInsert { index, class },
            ClassReplace { before, after } => ClassReplace {
                before: after,
                after: before,
            },
            ClassMove { from, to } => ClassMove { from: to, to: from },
            LayerInsert { index, layer } => LayerRemove { index, layer },
            LayerRemove { index, layer } => LayerInsert { index, layer },
            LayerPropsReplace { before, after } => LayerPropsReplace {
                before: after,
                after: before,
            },
            LayerMove { from, to } => LayerMove { from: to, to: from },
            StateReplace { before, after } => StateReplace {
                before: after,
                after: before,
            },
        }
    }

    pub fn apply(&self, doc: &mut Document) -> Result<(), CoreError> {
        use Patch::*;
        match self {
            ObjectInsert {
                layer,
                index,
                object,
            } => {
                let li = doc
                    .layer_index(layer)
                    .ok_or_else(|| CoreError::LayerNotFound(layer.to_string()))?;
                let objs = &mut doc.design_layers[li].objects;
                objs.insert((*index).min(objs.len()), object.clone());
            }
            ObjectRemove { layer, object, .. } => {
                let li = doc
                    .layer_index(layer)
                    .ok_or_else(|| CoreError::LayerNotFound(layer.to_string()))?;
                let objs = &mut doc.design_layers[li].objects;
                let oi = objs
                    .iter()
                    .position(|o| o.id == object.id)
                    .ok_or_else(|| CoreError::ObjectNotFound(object.id.to_string()))?;
                objs.remove(oi);
            }
            ObjectReplace { before, after } => {
                let (li, oi) = doc
                    .find_object(&before.id)
                    .ok_or_else(|| CoreError::ObjectNotFound(before.id.to_string()))?;
                doc.design_layers[li].objects[oi] = after.clone();
            }
            ClassInsert { index, class } => {
                let i = (*index).min(doc.classes.len());
                doc.classes.insert(i, class.clone());
            }
            ClassRemove { class, .. } => {
                let i = doc
                    .class_index(&class.id)
                    .ok_or_else(|| CoreError::ClassNotFound(class.id.to_string()))?;
                doc.classes.remove(i);
            }
            ClassReplace { before, after } => {
                let i = doc
                    .class_index(&before.id)
                    .ok_or_else(|| CoreError::ClassNotFound(before.id.to_string()))?;
                doc.classes[i] = after.clone();
            }
            ClassMove { from, to } => move_item(&mut doc.classes, *from, *to),
            LayerInsert { index, layer } => {
                let i = (*index).min(doc.design_layers.len());
                doc.design_layers.insert(i, layer.clone());
            }
            LayerRemove { layer, .. } => {
                let i = doc
                    .layer_index(&layer.props.id)
                    .ok_or_else(|| CoreError::LayerNotFound(layer.props.id.to_string()))?;
                doc.design_layers.remove(i);
            }
            LayerPropsReplace { before, after } => {
                let i = doc
                    .layer_index(&before.id)
                    .ok_or_else(|| CoreError::LayerNotFound(before.id.to_string()))?;
                doc.design_layers[i].props = after.clone();
            }
            LayerMove { from, to } => move_item(&mut doc.design_layers, *from, *to),
            StateReplace { after, .. } => doc.state = after.clone(),
        }
        Ok(())
    }
}
