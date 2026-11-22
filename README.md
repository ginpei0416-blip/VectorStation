# VectorStation(仮称)
Vectorworks と同じ操作感を持つ、自作の2D/3D CAD。個人利用を前提に開発中。
- 運用ルール: CLAUDE.md
- 作る順番: docs/roadmap.md
- 進捗: docs/progress.md
- 人間向けの全体管理は Notion で行う。

## 動かし方(開発者向け)
- テスト: `cargo test`
- 見本の実行: `cargo run -p vs-cli -- run --new examples/basic-plan.json plan.vws`
- 中身の表示: `cargo run -p vs-cli -- info plan.vws`
