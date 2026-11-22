//! VectorStation のコマンドライン。すべての操作を JSON で実行できる入口。
//! AI はここから動作確認する。人も、画面ができる前の確認に使える。
//!
//! 使い方:
//!   vs new <保存先.vws>                       新しい書類を作る
//!   vs info <ファイル.vws>                    中身の概要を表示する
//!   vs run <ファイル.vws|--new> <手順.json> [保存先.vws]
//!                                             手順(コマンドの並び)を実行する
//!   vs resolve <ファイル.vws> <図形ID>        図形の最終的な属性を表示する
//!
//! 手順ファイルは JSON の配列。各要素はコマンド({"cmd": "addObject", ...})か、
//! {"cmd": "undo"} / {"cmd": "redo"} / {"group": "名前", "cmds": [...]}(1回の Undo でまとめて戻す)。

use serde_json::Value;
use std::process::ExitCode;
use vs_core::{Command, Document, History, Id};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let r = match args.first().map(String::as_str) {
        Some("new") if args.len() == 2 => cmd_new(&args[1]),
        Some("info") if args.len() == 2 => cmd_info(&args[1]),
        Some("run") if args.len() == 3 || args.len() == 4 => {
            cmd_run(&args[1], &args[2], args.get(3))
        }
        Some("resolve") if args.len() == 3 => cmd_resolve(&args[1], &args[2]),
        _ => Err(USAGE.to_string()),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("エラー: {e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "使い方:
  vs new <保存先.vws>
  vs info <ファイル.vws>
  vs run <ファイル.vws|--new> <手順.json> [保存先.vws]
  vs resolve <ファイル.vws> <図形ID>";

fn load(path: &str) -> Result<Document, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("{path} を開けません: {e}"))?;
    vs_io::load_from_str(&s).map_err(|e| format!("{path}: {e}"))
}

fn save(doc: &Document, path: &str) -> Result<(), String> {
    let s = vs_io::save_to_string(doc).map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| format!("{path} に保存できません: {e}"))
}

fn cmd_new(out: &str) -> Result<(), String> {
    save(&Document::new(), out)?;
    println!("新しい書類を作りました: {out}");
    Ok(())
}

fn vis(v: vs_core::Visibility) -> &'static str {
    match v {
        vs_core::Visibility::Visible => "表示",
        vs_core::Visibility::Grayed => "グレー表示",
        vs_core::Visibility::Hidden => "非表示",
    }
}

fn cmd_info(path: &str) -> Result<(), String> {
    let doc = load(path)?;
    println!("書類: {path}");
    println!("クラス({}):", doc.classes.len());
    for c in &doc.classes {
        let mark = if c.builtin.is_some() { "(必須)" } else { "" };
        println!("  - {}{} [{}] id={}", c.name, mark, vis(c.visibility), c.id);
    }
    println!("デザインレイヤ({}):", doc.design_layers.len());
    for l in &doc.design_layers {
        let p = &l.props;
        let lock = if p.locked { " ロック" } else { "" };
        println!(
            "  - {} 1:{} [{}]{} 図形{}個 id={}",
            p.name,
            p.scale,
            vis(p.visibility),
            lock,
            l.objects.len(),
            p.id
        );
        for o in &l.objects {
            let cls = doc
                .class(&o.class_id)
                .map(|c| c.name.as_str())
                .unwrap_or("?");
            println!("      ・{} クラス={} id={}", o.shape.kind(), cls, o.id);
        }
    }
    let name = |id: &Option<Id>, f: &dyn Fn(&Id) -> Option<String>| {
        id.as_ref().and_then(f).unwrap_or("-".into())
    };
    println!(
        "アクティブ: レイヤ={} クラス={}",
        name(&doc.state.active_layer, &|i| doc
            .layer(i)
            .map(|l| l.props.name.clone())),
        name(&doc.state.active_class, &|i| doc
            .class(i)
            .map(|c| c.name.clone()))
    );
    Ok(())
}

fn cmd_run(input: &str, script: &str, out: Option<&String>) -> Result<(), String> {
    let mut doc = if input == "--new" {
        Document::new()
    } else {
        load(input)?
    };
    let text =
        std::fs::read_to_string(script).map_err(|e| format!("{script} を開けません: {e}"))?;
    let steps: Vec<Value> =
        serde_json::from_str(&text).map_err(|e| format!("{script} が読めません: {e}"))?;
    let mut hist = History::new();
    for (i, step) in steps.iter().enumerate() {
        let n = i + 1;
        let result = if let Some(label) = step.get("group").and_then(Value::as_str) {
            let cmds: Vec<Command> =
                serde_json::from_value(step.get("cmds").cloned().unwrap_or(Value::Null))
                    .map_err(|e| format!("手順{n}: まとまりの中身が読めません: {e}"))?;
            hist.execute_group(&mut doc, label, &cmds)
                .map(|ids| format!("まとまり「{label}」 {}", fmt_ids(&ids)))
        } else {
            match step.get("cmd").and_then(Value::as_str) {
                Some("undo") => hist.undo(&mut doc).map(|l| format!("取り消し: {l}")),
                Some("redo") => hist.redo(&mut doc).map(|l| format!("やり直し: {l}")),
                _ => {
                    let cmd: Command = serde_json::from_value(step.clone())
                        .map_err(|e| format!("手順{n}: コマンドが読めません: {e}"))?;
                    let name = step["cmd"].as_str().unwrap_or("?").to_string();
                    hist.execute(&mut doc, &cmd)
                        .map(|ids| format!("{name} {}", fmt_ids(&ids)))
                }
            }
        };
        match result {
            Ok(msg) => println!("手順{n} OK: {msg}"),
            Err(e) => return Err(format!("手順{n} で止まりました: {e}")),
        }
    }
    if let Some(out) = out {
        save(&doc, out)?;
        println!("保存しました: {out}");
    }
    Ok(())
}

fn fmt_ids(ids: &[Id]) -> String {
    if ids.is_empty() {
        String::new()
    } else {
        format!(
            "→ {}",
            ids.iter()
                .map(|i| i.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn cmd_resolve(path: &str, id: &str) -> Result<(), String> {
    let doc = load(path)?;
    let r = doc
        .resolved_attributes(&Id::from(id))
        .ok_or_else(|| format!("図形が見つかりません: {id}"))?;
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
    Ok(())
}
