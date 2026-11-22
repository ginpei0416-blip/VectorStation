//! Undo/Redo。コマンドの実行結果(変更の記録)を積み重ねる。

use crate::command::Command;
use crate::error::CoreError;
use crate::id::Id;
use crate::model::Document;
use crate::patch::Patch;

#[derive(Debug, Clone)]
struct Step {
    label: String,
    patches: Vec<Patch>,
}

#[derive(Debug, Default, Clone)]
pub struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    /// コマンドを実行して記録する。新しく作られたIDを返す。
    pub fn execute(&mut self, doc: &mut Document, cmd: &Command) -> Result<Vec<Id>, CoreError> {
        self.execute_group(doc, &label_of(cmd), std::slice::from_ref(cmd))
    }

    /// 複数のコマンドを、1回の Undo で戻せるまとまりとして実行する。
    /// 途中で失敗したら、まとまり全体を取り消す。
    pub fn execute_group(
        &mut self,
        doc: &mut Document,
        label: &str,
        cmds: &[Command],
    ) -> Result<Vec<Id>, CoreError> {
        let mut patches = vec![];
        let mut created = vec![];
        for c in cmds {
            match c.execute(doc) {
                Ok((p, ids)) => {
                    patches.extend(p);
                    created.extend(ids);
                }
                Err(e) => {
                    for p in patches.iter().rev() {
                        let _ = Patch::inverse(p).apply(doc);
                    }
                    return Err(e);
                }
            }
        }
        if !patches.is_empty() {
            self.undo.push(Step {
                label: label.to_string(),
                patches,
            });
            self.redo.clear();
        }
        Ok(created)
    }

    /// 取り消し。取り消した操作の名前を返す。
    pub fn undo(&mut self, doc: &mut Document) -> Result<String, CoreError> {
        let step = self.undo.pop().ok_or(CoreError::NothingToUndo)?;
        for p in step.patches.iter().rev() {
            p.inverse().apply(doc)?;
        }
        let label = step.label.clone();
        self.redo.push(step);
        Ok(label)
    }

    /// やり直し。
    pub fn redo(&mut self, doc: &mut Document) -> Result<String, CoreError> {
        let step = self.redo.pop().ok_or(CoreError::NothingToRedo)?;
        for p in &step.patches {
            p.apply(doc)?;
        }
        let label = step.label.clone();
        self.undo.push(step);
        Ok(label)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

fn label_of(cmd: &Command) -> String {
    serde_json::to_value(cmd)
        .ok()
        .and_then(|v| v.get("cmd").and_then(|c| c.as_str()).map(String::from))
        .unwrap_or_else(|| "操作".into())
}
