//! 保存・読込の動作確認。

use serde_json::{json, Value};
use vs_core::{Document, Id};
use vs_io::{load_from_str, save_to_string, IoError, CURRENT_FORMAT_VERSION};

const V1_BASIC: &str = include_str!("fixtures/v1-basic.vws");

#[test]
fn 形式1の見本ファイルが開ける_この確認は今後ずっと残す() {
    let d = load_from_str(V1_BASIC).unwrap();
    assert_eq!(d.classes.len(), 6);
    assert_eq!(d.design_layers.len(), 2);
    assert_eq!(d.design_layers[1].props.scale, 10.0);
    assert_eq!(
        d.resolved_attributes(&Id::from("obj-grid-x1"))
            .unwrap()
            .line_weight,
        0.18
    );
}

#[test]
fn 保存して開き直すと同じ書類に戻る() {
    let d = load_from_str(V1_BASIC).unwrap();
    let s = save_to_string(&d).unwrap();
    assert_eq!(load_from_str(&s).unwrap(), d);
    // 2回目の保存結果も1回目と一字一句同じ(余計な変化が起きない)
    assert_eq!(save_to_string(&load_from_str(&s).unwrap()).unwrap(), s);
}

#[test]
fn 保存したファイルには形式の版が書かれる() {
    let s = save_to_string(&Document::new()).unwrap();
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["formatVersion"], json!(CURRENT_FORMAT_VERSION));
    assert_eq!(v["app"], json!("VectorStation"));
}

/// 将来の版で増えた項目や、まだ対応していない図形・シートレイヤ・リソースを、消さずに書き戻す。
#[test]
fn まだ知らない項目や図形も消さずに保存し直す() {
    let mut v: Value = serde_json::from_str(V1_BASIC).unwrap();
    v["futureTopLevel"] = json!("残る");
    let doc = &mut v["document"];
    doc["futureDocField"] = json!({"a": 1});
    doc["classes"][2]["futureClassField"] = json!(true);
    doc["designLayers"][0]["futureLayerField"] = json!([1, 2]);
    doc["designLayers"][0]["objects"][0]["records"] =
        json!([{"format": "仕上", "values": {"品番": "AB-1"}}]);
    doc["designLayers"][0]["objects"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "obj-wall-bim", "type": "wall", "classId": "cls-none", "geometry": {"thickness": 120}}));
    doc["sheetLayers"] = json!([{"id": "sht-1", "number": "A-01", "viewports": []}]);
    doc["resources"] = json!({"hatches": [{"id": "h-1", "name": "コンクリート"}]});
    let text = serde_json::to_string(&v).unwrap();

    let d = load_from_str(&text).unwrap();
    let out: Value = serde_json::from_str(&save_to_string(&d).unwrap()).unwrap();
    let od = &out["document"];
    assert_eq!(od["futureDocField"], json!({"a": 1}));
    assert_eq!(od["classes"][2]["futureClassField"], json!(true));
    assert_eq!(od["designLayers"][0]["futureLayerField"], json!([1, 2]));
    assert_eq!(
        od["designLayers"][0]["objects"][0]["records"][0]["values"]["品番"],
        json!("AB-1")
    );
    let wall = od["designLayers"][0]["objects"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(wall["type"], json!("wall"));
    assert_eq!(wall["geometry"]["thickness"], json!(120));
    assert_eq!(od["sheetLayers"][0]["number"], json!("A-01"));
    assert_eq!(od["resources"]["hatches"][0]["name"], json!("コンクリート"));
}

#[test]
fn 新しすぎる版のファイルはわかる言葉で断る() {
    let mut v: Value = serde_json::from_str(V1_BASIC).unwrap();
    v["formatVersion"] = json!(CURRENT_FORMAT_VERSION + 1);
    let e = load_from_str(&v.to_string()).unwrap_err();
    assert!(matches!(e, IoError::TooNew { .. }));
    assert!(e.to_string().contains("新しい版"));
}

#[test]
fn 壊れたファイルや必須クラスのないファイルは断る() {
    assert!(matches!(load_from_str("{").unwrap_err(), IoError::Json(_)));
    assert!(matches!(
        load_from_str("{}").unwrap_err(),
        IoError::NotVectorStation
    ));
    let mut v: Value = serde_json::from_str(V1_BASIC).unwrap();
    v["document"]["classes"].as_array_mut().unwrap().remove(0);
    assert!(matches!(
        load_from_str(&v.to_string()).unwrap_err(),
        IoError::Invalid(_)
    ));
}

#[test]
fn idが重複したファイルは断る() {
    let mut v: Value = serde_json::from_str(V1_BASIC).unwrap();
    let first = v["document"]["designLayers"][0]["objects"][0].clone();
    v["document"]["designLayers"][0]["objects"]
        .as_array_mut()
        .unwrap()
        .push(first);
    assert!(matches!(
        load_from_str(&v.to_string()).unwrap_err(),
        IoError::Invalid(_)
    ));
}
