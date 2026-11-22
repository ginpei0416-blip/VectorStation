//! 中核の動作確認。各テストの名前は、作成者が読んでわかる言葉で書く。

use serde_json::json;
use vs_core::attributes::ClassAttributes;
use vs_core::{
    Attr, Attributes, Color, Command, CoreError, Document, History, Id, LineType, Visibility,
};

fn cmd(v: serde_json::Value) -> Command {
    serde_json::from_value(v).expect("コマンドの書き方が正しいこと")
}

fn line(id: &str, class: Option<&str>) -> Command {
    let mut v = json!({"cmd":"addObject","id":id,
        "shape":{"type":"line","geometry":{"from":[0,0],"to":[6000,0]}}});
    if let Some(c) = class {
        v["class"] = json!(c);
    }
    cmd(v)
}

/// 通り芯クラス: 一点鎖線(線種リソース参照)、0.18mm、赤、作成時にクラスの属性を使う
fn grid_class() -> Command {
    cmd(
        json!({"cmd":"addClass","id":"cls-grid","name":"00_基準線","useAtCreation":true,
        "attributes":{"lineType":{"kind":"resource","resource":"lt-dashdot"},"lineWeight":0.18,"penColor":[255,0,0]}}),
    )
}

fn setup() -> (Document, History) {
    (Document::new(), History::new())
}

#[test]
fn 新しい書類には一般と寸法クラスと1_50のレイヤが1枚ある() {
    let d = Document::new();
    let names: Vec<_> = d.classes.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["一般", "寸法"]);
    assert_eq!(d.design_layers.len(), 1);
    assert_eq!(d.design_layers[0].props.scale, 50.0);
}

#[test]
fn 作成時にクラスの属性を使うクラスで描いた線はクラスの見た目になる() {
    let (mut d, mut h) = setup();
    h.execute(&mut d, &grid_class()).unwrap();
    h.execute(&mut d, &line("obj-1", Some("cls-grid"))).unwrap();
    let r = d.resolved_attributes(&Id::from("obj-1")).unwrap();
    assert_eq!(r.line_type, LineType::Resource(Id::from("lt-dashdot")));
    assert_eq!(r.line_weight, 0.18);
    assert_eq!(r.pen_color, Color(255, 0, 0));
}

#[test]
fn クラスの属性を変えるとクラスに従う図形の見た目が一括で変わる() {
    let (mut d, mut h) = setup();
    h.execute(&mut d, &grid_class()).unwrap();
    h.execute(&mut d, &line("obj-1", Some("cls-grid"))).unwrap();
    h.execute(&mut d, &line("obj-2", Some("cls-grid"))).unwrap();
    let mut a = d.class(&Id::from("cls-grid")).unwrap().attributes.clone();
    a.pen_color = Some(Color(0, 0, 255));
    h.execute(
        &mut d,
        &Command::SetClass {
            id: "cls-grid".into(),
            name: None,
            visibility: None,
            attributes: Some(a),
            use_at_creation: None,
        },
    )
    .unwrap();
    for id in ["obj-1", "obj-2"] {
        assert_eq!(
            d.resolved_attributes(&Id::from(id)).unwrap().pen_color,
            Color(0, 0, 255)
        );
    }
}

#[test]
fn 直接指定した項目はクラスより優先され_他の項目はクラスに従ったまま() {
    let (mut d, mut h) = setup();
    h.execute(&mut d, &grid_class()).unwrap();
    h.execute(&mut d, &line("obj-1", Some("cls-grid"))).unwrap();
    let a = Attributes {
        pen_color: Some(Attr::value(Color(0, 128, 0))),
        ..Default::default()
    };
    h.execute(
        &mut d,
        &Command::SetObjectAttributes {
            ids: vec!["obj-1".into()],
            attributes: a,
        },
    )
    .unwrap();
    let r = d.resolved_attributes(&Id::from("obj-1")).unwrap();
    assert_eq!(r.pen_color, Color(0, 128, 0), "色は直接指定");
    assert_eq!(r.line_weight, 0.18, "太さはクラスに従う");
}

#[test]
fn 普通のクラスで描いた図形は図形ごとの直接指定になる() {
    let (mut d, mut h) = setup();
    h.execute(
        &mut d,
        &cmd(json!({"cmd":"addClass","id":"cls-wall","name":"01_壁-新設",
        "attributes":{"lineWeight":0.35}})),
    )
    .unwrap();
    h.execute(&mut d, &line("obj-1", Some("cls-wall"))).unwrap();
    // 作成時にクラスの属性を使わないので、クラスの 0.35 ではなく既定値になる(Vectorworks と同じ)
    assert_eq!(
        d.resolved_attributes(&Id::from("obj-1"))
            .unwrap()
            .line_weight,
        0.13
    );
}

#[test]
fn クラス名を変えても図形とのつながりは切れない() {
    let (mut d, mut h) = setup();
    h.execute(&mut d, &grid_class()).unwrap();
    h.execute(&mut d, &line("obj-1", Some("cls-grid"))).unwrap();
    h.execute(
        &mut d,
        &Command::SetClass {
            id: "cls-grid".into(),
            name: Some("通り芯".into()),
            visibility: None,
            attributes: None,
            use_at_creation: None,
        },
    )
    .unwrap();
    assert_eq!(
        d.object(&Id::from("obj-1")).unwrap().class_id,
        Id::from("cls-grid")
    );
    assert_eq!(
        d.resolved_attributes(&Id::from("obj-1"))
            .unwrap()
            .line_weight,
        0.18
    );
}

#[test]
fn 一般と寸法クラスは削除できない() {
    let (mut d, mut h) = setup();
    for id in ["cls-none", "cls-dimension"] {
        let e = h
            .execute(&mut d, &Command::DeleteClass { id: id.into() })
            .unwrap_err();
        assert!(matches!(e, CoreError::BuiltinClass(_)));
    }
}

#[test]
fn クラスを削除すると図形は一般クラスに移り_取り消すと元に戻る() {
    let (mut d, mut h) = setup();
    h.execute(&mut d, &grid_class()).unwrap();
    h.execute(&mut d, &line("obj-1", Some("cls-grid"))).unwrap();
    let before = d.clone();
    h.execute(
        &mut d,
        &Command::DeleteClass {
            id: "cls-grid".into(),
        },
    )
    .unwrap();
    assert_eq!(
        d.object(&Id::from("obj-1")).unwrap().class_id,
        Id::from("cls-none")
    );
    assert!(d.class(&Id::from("cls-grid")).is_none());
    h.undo(&mut d).unwrap();
    assert_eq!(d, before);
}

#[test]
fn 数値移動cmd_mで図形が指定量だけ動く() {
    let (mut d, mut h) = setup();
    h.execute(&mut d, &line("obj-1", None)).unwrap();
    h.execute(
        &mut d,
        &cmd(json!({"cmd":"moveObjects","ids":["obj-1"],"dx":100,"dy":-50})),
    )
    .unwrap();
    let o = d.object(&Id::from("obj-1")).unwrap();
    assert_eq!(
        o.shape,
        vs_core::Shape::Line {
            from: [100.0, -50.0],
            to: [6100.0, -50.0]
        }
    );
}

#[test]
fn 複製cmd_dで新しい図形ができ_元の図形はそのまま() {
    let (mut d, mut h) = setup();
    h.execute(&mut d, &grid_class()).unwrap();
    h.execute(&mut d, &line("obj-1", Some("cls-grid"))).unwrap();
    let ids = h
        .execute(
            &mut d,
            &cmd(json!({"cmd":"duplicateObjects","ids":["obj-1"],"dx":0,"dy":3600})),
        )
        .unwrap();
    assert_eq!(ids.len(), 1);
    assert_ne!(ids[0], Id::from("obj-1"));
    let copy = d.object(&ids[0]).unwrap();
    assert_eq!(
        copy.class_id,
        Id::from("cls-grid"),
        "クラスも属性も引き継ぐ"
    );
    assert_eq!(
        copy.shape,
        vs_core::Shape::Line {
            from: [0.0, 3600.0],
            to: [6000.0, 3600.0]
        }
    );
    assert_eq!(
        d.object(&Id::from("obj-1")).unwrap().shape,
        vs_core::Shape::Line {
            from: [0.0, 0.0],
            to: [6000.0, 0.0]
        }
    );
}

#[test]
fn ロックしたレイヤには描けず_ロックした図形は動かせない() {
    let (mut d, mut h) = setup();
    h.execute(&mut d, &line("obj-1", None)).unwrap();
    h.execute(
        &mut d,
        &cmd(json!({"cmd":"setObjectLocked","ids":["obj-1"],"locked":true})),
    )
    .unwrap();
    let e = h
        .execute(
            &mut d,
            &cmd(json!({"cmd":"moveObjects","ids":["obj-1"],"dx":1,"dy":1})),
        )
        .unwrap_err();
    assert!(matches!(e, CoreError::ObjectLocked(_)));
    h.execute(
        &mut d,
        &cmd(json!({"cmd":"setDesignLayer","id":"lay-1","locked":true})),
    )
    .unwrap();
    let e = h.execute(&mut d, &line("obj-2", None)).unwrap_err();
    assert!(matches!(e, CoreError::LayerLocked(_)));
}

#[test]
fn 途中で失敗した操作は書類を何も変えない() {
    let (mut d, mut h) = setup();
    h.execute(&mut d, &line("obj-1", None)).unwrap();
    let before = d.clone();
    // 1つ目は動かせるが、2つ目が存在しない
    let e = h
        .execute(
            &mut d,
            &cmd(json!({"cmd":"moveObjects","ids":["obj-1","obj-x"],"dx":10,"dy":0})),
        )
        .unwrap_err();
    assert!(matches!(e, CoreError::ObjectNotFound(_)));
    assert_eq!(d, before);
}

#[test]
fn すべての操作を取り消すと最初の状態に戻り_やり直すと最後の状態に戻る() {
    let (mut d, mut h) = setup();
    let start = d.clone();
    let steps = vec![
        grid_class(),
        cmd(json!({"cmd":"addDesignLayer","id":"lay-2","name":"2F平面","scale":100})),
        line("obj-1", Some("cls-grid")),
        cmd(
            json!({"cmd":"addObject","id":"obj-2","shape":{"type":"rect","geometry":{"center":[0,0],"width":900,"height":600}}}),
        ),
        cmd(json!({"cmd":"moveObjects","ids":["obj-1","obj-2"],"dx":5,"dy":5})),
        cmd(json!({"cmd":"duplicateObjects","ids":["obj-2"],"dx":1000})),
        cmd(json!({"cmd":"setClass","id":"cls-grid","visibility":"grayed"})),
        cmd(json!({"cmd":"moveClass","id":"cls-grid","to":0})),
        cmd(json!({"cmd":"moveDesignLayer","id":"lay-2","to":0})),
        cmd(json!({"cmd":"deleteObjects","ids":["obj-1"]})),
        cmd(json!({"cmd":"deleteClass","id":"cls-grid"})),
        cmd(json!({"cmd":"deleteDesignLayer","id":"lay-1"})),
    ];
    for s in &steps {
        h.execute(&mut d, s).unwrap();
    }
    let end = d.clone();
    while h.can_undo() {
        h.undo(&mut d).unwrap();
    }
    assert_eq!(d, start);
    while h.can_redo() {
        h.redo(&mut d).unwrap();
    }
    assert_eq!(d, end);
}

#[test]
fn まとめた操作は1回の取り消しで全部戻る() {
    let (mut d, mut h) = setup();
    h.execute(&mut d, &line("obj-1", None)).unwrap();
    let before = d.clone();
    let ids = h
        .execute_group(
            &mut d,
            "複製して移動",
            &[cmd(json!({"cmd":"duplicateObjects","ids":["obj-1"]}))],
        )
        .unwrap();
    h.execute_group(
        &mut d,
        "続き",
        &[cmd(
            json!({"cmd":"moveObjects","ids":[ids[0].as_str()],"dx":0,"dy":10}),
        )],
    )
    .unwrap();
    h.undo(&mut d).unwrap();
    h.undo(&mut d).unwrap();
    assert_eq!(d, before);
}

#[test]
fn 新しいレイヤはアクティブになり_縮尺はレイヤごとに持てる() {
    let (mut d, mut h) = setup();
    h.execute(
        &mut d,
        &cmd(json!({"cmd":"addDesignLayer","id":"lay-detail","name":"詳細","scale":10})),
    )
    .unwrap();
    assert_eq!(d.state.active_layer, Some(Id::from("lay-detail")));
    assert_eq!(d.layer(&Id::from("lay-detail")).unwrap().props.scale, 10.0);
    assert_eq!(d.layer(&Id::from("lay-1")).unwrap().props.scale, 50.0);
    h.execute(&mut d, &line("obj-1", None)).unwrap();
    assert_eq!(
        d.find_object(&Id::from("obj-1")).unwrap().0,
        1,
        "アクティブなレイヤに描かれる"
    );
}

#[test]
fn おかしな値は受け付けない() {
    let (mut d, mut h) = setup();
    assert!(matches!(
        h.execute(
            &mut d,
            &cmd(json!({"cmd":"addDesignLayer","name":"x","scale":0}))
        )
        .unwrap_err(),
        CoreError::InvalidScale(_)
    ));
    assert!(matches!(
        h.execute(&mut d, &cmd(json!({"cmd":"addClass","name":"  "})))
            .unwrap_err(),
        CoreError::EmptyName
    ));
    h.execute(&mut d, &line("obj-1", None)).unwrap();
    let e = h
        .execute(&mut d, &cmd(json!({"cmd":"setObjectAttributes","ids":["obj-1"],"attributes":{"fillOpacity":{"value":150}}})))
        .unwrap_err();
    assert!(matches!(e, CoreError::InvalidValue(_)));
    assert!(matches!(
        h.execute(
            &mut d,
            &cmd(json!({"cmd":"deleteDesignLayer","id":"lay-1"}))
        )
        .unwrap_err(),
        CoreError::LastLayer
    ));
}

#[test]
fn 表示状態は表示とグレー表示と非表示の3つ() {
    let (mut d, mut h) = setup();
    for v in ["visible", "grayed", "hidden"] {
        h.execute(
            &mut d,
            &cmd(json!({"cmd":"setDesignLayer","id":"lay-1","visibility":v})),
        )
        .unwrap();
    }
    assert_eq!(d.design_layers[0].props.visibility, Visibility::Hidden);
    let _ = ClassAttributes::default();
}
