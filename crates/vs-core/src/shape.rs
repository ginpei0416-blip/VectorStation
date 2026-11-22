//! 図形の形。座標はすべて実寸(mm)。X 右・Y 上。

use serde::{Deserialize, Serialize};

/// 点 [x, y](mm)。
pub type Point = [f64; 2];

/// ポリラインの頂点の種類(Vectorworks の頂点の種類に合わせる)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum VertexKind {
    #[default]
    Corner,
    Bezier,
    Cubic,
    Arc {
        #[serde(skip_serializing_if = "Option::is_none")]
        radius: Option<f64>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vertex {
    pub p: Point,
    #[serde(default, skip_serializing_if = "is_corner")]
    pub vertex: VertexKind,
}

fn is_corner(k: &VertexKind) -> bool {
    *k == VertexKind::Corner
}

/// 図形の形。ファイルでは `"type"` と `"geometry"` に分けて保存する。
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Line {
        from: Point,
        to: Point,
    },
    /// 長方形: 中心、幅、高さ、回転角(度)
    Rect {
        center: Point,
        width: f64,
        height: f64,
        rotation: f64,
    },
    /// 円・楕円: 中心、横半径、縦半径、回転角(度)。円は横半径=縦半径
    Ellipse {
        center: Point,
        rx: f64,
        ry: f64,
        rotation: f64,
    },
    Polyline {
        vertices: Vec<Vertex>,
        closed: bool,
    },
    /// まだ対応していない種類。中身をそのまま保持して書き戻す
    Unknown {
        kind: String,
        geometry: serde_json::Value,
    },
}

#[derive(Serialize, Deserialize)]
struct LineG {
    from: Point,
    to: Point,
}
#[derive(Serialize, Deserialize)]
struct RectG {
    center: Point,
    width: f64,
    height: f64,
    #[serde(default)]
    rotation: f64,
}
#[derive(Serialize, Deserialize)]
struct EllipseG {
    center: Point,
    rx: f64,
    ry: f64,
    #[serde(default)]
    rotation: f64,
}
#[derive(Serialize, Deserialize)]
struct PolylineG {
    vertices: Vec<Vertex>,
    #[serde(default)]
    closed: bool,
}

impl Shape {
    pub fn kind(&self) -> &str {
        match self {
            Shape::Line { .. } => "line",
            Shape::Rect { .. } => "rect",
            Shape::Ellipse { .. } => "ellipse",
            Shape::Polyline { .. } => "polyline",
            Shape::Unknown { kind, .. } => kind,
        }
    }

    /// `type` と `geometry` から形を作る。知らない種類は Unknown として保持する。
    /// 知っている種類なのに中身が壊れている場合はエラー。
    pub fn from_parts(kind: &str, geometry: serde_json::Value) -> Result<Shape, String> {
        use serde_json::from_value as fv;
        let e = |err: serde_json::Error| format!("図形「{kind}」の形が読めません: {err}");
        Ok(match kind {
            "line" => {
                let g: LineG = fv(geometry).map_err(e)?;
                Shape::Line {
                    from: g.from,
                    to: g.to,
                }
            }
            "rect" => {
                let g: RectG = fv(geometry).map_err(e)?;
                Shape::Rect {
                    center: g.center,
                    width: g.width,
                    height: g.height,
                    rotation: g.rotation,
                }
            }
            "ellipse" => {
                let g: EllipseG = fv(geometry).map_err(e)?;
                Shape::Ellipse {
                    center: g.center,
                    rx: g.rx,
                    ry: g.ry,
                    rotation: g.rotation,
                }
            }
            "polyline" => {
                let g: PolylineG = fv(geometry).map_err(e)?;
                Shape::Polyline {
                    vertices: g.vertices,
                    closed: g.closed,
                }
            }
            other => Shape::Unknown {
                kind: other.to_string(),
                geometry,
            },
        })
    }

    /// `type` と `geometry` に分ける。
    pub fn to_parts(&self) -> (String, serde_json::Value) {
        use serde_json::to_value as tv;
        let g = match self {
            Shape::Line { from, to } => tv(LineG {
                from: *from,
                to: *to,
            }),
            Shape::Rect {
                center,
                width,
                height,
                rotation,
            } => tv(RectG {
                center: *center,
                width: *width,
                height: *height,
                rotation: *rotation,
            }),
            Shape::Ellipse {
                center,
                rx,
                ry,
                rotation,
            } => tv(EllipseG {
                center: *center,
                rx: *rx,
                ry: *ry,
                rotation: *rotation,
            }),
            Shape::Polyline { vertices, closed } => tv(PolylineG {
                vertices: vertices.clone(),
                closed: *closed,
            }),
            Shape::Unknown { geometry, .. } => Ok(geometry.clone()),
        }
        .expect("形の書き出しに失敗");
        (self.kind().to_string(), g)
    }

    /// 平行移動(⌘M の数値移動、⌘D の複製のずらし)。未対応の図形は動かせない。
    pub fn translated(&self, dx: f64, dy: f64) -> Option<Shape> {
        let mv = |p: &Point| [p[0] + dx, p[1] + dy];
        Some(match self {
            Shape::Line { from, to } => Shape::Line {
                from: mv(from),
                to: mv(to),
            },
            Shape::Rect {
                center,
                width,
                height,
                rotation,
            } => Shape::Rect {
                center: mv(center),
                width: *width,
                height: *height,
                rotation: *rotation,
            },
            Shape::Ellipse {
                center,
                rx,
                ry,
                rotation,
            } => Shape::Ellipse {
                center: mv(center),
                rx: *rx,
                ry: *ry,
                rotation: *rotation,
            },
            Shape::Polyline { vertices, closed } => Shape::Polyline {
                vertices: vertices
                    .iter()
                    .map(|v| Vertex {
                        p: mv(&v.p),
                        vertex: v.vertex.clone(),
                    })
                    .collect(),
                closed: *closed,
            },
            Shape::Unknown { .. } => return None,
        })
    }
}
