//! 属性(線の色・太さ・線種、面など)。
//! 各項目は「クラスに従う」か「直接指定」かを選べる(Vectorworks と同じ考え方)。

use crate::id::Id;
use serde::{Deserialize, Serialize};

/// 色(RGB、0〜255)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    pub const BLACK: Color = Color(0, 0, 0);
    pub const WHITE: Color = Color(255, 255, 255);
}

/// 線種。実線か、線種リソースの参照。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "resource")]
pub enum LineType {
    Solid,
    Resource(Id),
}

/// 面の塗り。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "resource")]
pub enum Fill {
    None,
    Solid,
    Pattern(Id),
    Hatch(Id),
    Tile(Id),
}

/// 1つの属性項目の値:「クラスに従う」か「直接指定」。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Attr<T> {
    ByClass {
        #[serde(rename = "byClass")]
        by_class: ByClassMarker,
    },
    Value {
        value: T,
    },
}

/// `{"byClass": true}` を表すための印。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "bool", into = "bool")]
pub struct ByClassMarker;
impl TryFrom<bool> for ByClassMarker {
    type Error = &'static str;
    fn try_from(b: bool) -> Result<Self, Self::Error> {
        if b {
            Ok(ByClassMarker)
        } else {
            Err("byClass は true のみ")
        }
    }
}
impl From<ByClassMarker> for bool {
    fn from(_: ByClassMarker) -> bool {
        true
    }
}

impl<T> Attr<T> {
    pub fn by_class() -> Self {
        Attr::ByClass {
            by_class: ByClassMarker,
        }
    }
    pub fn value(v: T) -> Self {
        Attr::Value { value: v }
    }
    pub fn is_by_class(&self) -> bool {
        matches!(self, Attr::ByClass { .. })
    }
}

/// 図形の属性。項目が `None` のときは書類の既定値を使う。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pen_color: Option<Attr<Color>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pen_opacity: Option<Attr<u8>>,
    /// 線の太さ(紙の上の mm)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_weight: Option<Attr<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_type: Option<Attr<LineType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Attr<Fill>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Attr<Color>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_opacity: Option<Attr<u8>>,
}

/// クラスが持つ属性(クラスには「クラスに従う」はない)。`None` は書類の既定値。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pen_color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pen_opacity: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_weight: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_type: Option<LineType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_opacity: Option<u8>,
}

/// 最終的に画面や印刷で使う属性(すべて確定した値)。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolved {
    pub pen_color: Color,
    pub pen_opacity: u8,
    pub line_weight: f64,
    pub line_type: LineType,
    pub fill: Fill,
    pub fill_color: Color,
    pub fill_opacity: u8,
}

impl Default for Resolved {
    /// 書類の既定値(黒の実線 0.13mm、面は白の単色、不透明)。
    fn default() -> Self {
        Resolved {
            pen_color: Color::BLACK,
            pen_opacity: 100,
            line_weight: 0.13,
            line_type: LineType::Solid,
            fill: Fill::Solid,
            fill_color: Color::WHITE,
            fill_opacity: 100,
        }
    }
}

fn pick<T: Clone>(obj: &Option<Attr<T>>, class: &Option<T>, default: T) -> T {
    match obj {
        Some(Attr::Value { value }) => value.clone(),
        Some(Attr::ByClass { .. }) => class.clone().unwrap_or(default),
        None => default,
    }
}

impl Attributes {
    /// 「クラスに従う」を解決して、最終的な属性を求める。
    pub fn resolve(&self, class: &ClassAttributes) -> Resolved {
        let d = Resolved::default();
        Resolved {
            pen_color: pick(&self.pen_color, &class.pen_color, d.pen_color),
            pen_opacity: pick(&self.pen_opacity, &class.pen_opacity, d.pen_opacity),
            line_weight: pick(&self.line_weight, &class.line_weight, d.line_weight),
            line_type: pick(&self.line_type, &class.line_type, d.line_type),
            fill: pick(&self.fill, &class.fill, d.fill),
            fill_color: pick(&self.fill_color, &class.fill_color, d.fill_color),
            fill_opacity: pick(&self.fill_opacity, &class.fill_opacity, d.fill_opacity),
        }
    }

    /// 全項目を「クラスに従う」にした属性(「作成時にクラスの属性を使う」用)。
    pub fn all_by_class() -> Self {
        Attributes {
            pen_color: Some(Attr::by_class()),
            pen_opacity: Some(Attr::by_class()),
            line_weight: Some(Attr::by_class()),
            line_type: Some(Attr::by_class()),
            fill: Some(Attr::by_class()),
            fill_color: Some(Attr::by_class()),
            fill_opacity: Some(Attr::by_class()),
        }
    }

    /// `other` で指定された項目だけを上書きする。
    pub fn merged_with(&self, other: &Attributes) -> Attributes {
        let mut a = self.clone();
        macro_rules! m {
            ($f:ident) => {
                if other.$f.is_some() {
                    a.$f = other.$f.clone();
                }
            };
        }
        m!(pen_color);
        m!(pen_opacity);
        m!(line_weight);
        m!(line_type);
        m!(fill);
        m!(fill_color);
        m!(fill_opacity);
        a
    }
}
