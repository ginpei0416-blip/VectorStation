//! コマンドラインの動作確認。

use std::process::Command;

fn vs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vs"))
}

#[test]
fn 見本の手順を実行して保存し_中身を表示できる() {
    let dir = std::env::temp_dir().join(format!("vs-cli-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("plan.vws");
    let script = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/basic-plan.json"
    );
    let r = vs()
        .args(["run", "--new", script, out.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let r = vs().args(["info", out.to_str().unwrap()]).output().unwrap();
    let text = String::from_utf8(r.stdout).unwrap();
    assert!(text.contains("00_基準線"));
    assert!(text.contains("1F平面 1:50"));
    assert!(text.contains("詳細 1:10"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn 失敗した手順は何番目で止まったかを伝える() {
    let dir = std::env::temp_dir().join(format!("vs-cli-test-err-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("bad.json");
    std::fs::write(
        &script,
        r#"[{"cmd":"addClass","name":"A"},{"cmd":"deleteClass","id":"cls-none"}]"#,
    )
    .unwrap();
    let r = vs()
        .args(["run", "--new", script.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!r.status.success());
    let err = String::from_utf8(r.stderr).unwrap();
    assert!(err.contains("手順2"), "{err}");
    assert!(err.contains("削除できません"), "{err}");
    std::fs::remove_dir_all(&dir).ok();
}
