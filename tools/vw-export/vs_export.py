# -*- coding: utf-8 -*-
"""
VectorStation 書き出しスクリプト(第1版)
Vectorworks の中で実行し、開いている図面を VectorStation 形式(.vws)で書き出す。

- 使っているのは Vectorworks が公開しているスクリプト機能(vs モジュール)だけ。
  ファイル(.vwx)の中身を解析することはしない(docs/decisions.md D-001, D-006)。
- 書き出し先: 図面と同じフォルダに「図面名.vws」。
- 図面は一切変更しない(読むだけ)。

第1版で書き出すもの:
  クラス(表示状態、線の太さ・色・線種、面、不透明度、作成時にクラスの属性を使う)
  デザインレイヤ(縮尺、表示状態、高さ)
  図形: 直線、長方形、円・楕円、多角形、ポリライン
  図形ごとの属性(クラスに従う/直接指定)
まだ書き出せないもの(中身の一部を「未対応の図形」として保持し、一覧で報告する):
  円弧、文字、寸法、グループ、シンボル、プラグインオブジェクト、回転した長方形・楕円、ほか
"""

import json
import os

FORMAT_VERSION = 1
EXPORTER_VERSION = "0.1.0"

# Vectorworks の図形の種類番号(公開リファレンス Appendix D)
T_LINE, T_RECT, T_OVAL, T_POLY, T_ARC, T_TEXT, T_GROUP = 2, 3, 4, 5, 6, 10, 11
T_POLYLINE, T_LAYER, T_DIMENSION, T_SYMBOL, T_PLUGIN, T_VIEWPORT = 21, 31, 63, 15, 86, 122
T_HATCH, T_TILE, T_IMAGEFILL, T_GRADIENT, T_LINETYPE = 66, 108, 119, 120, 96
TYPE_NAMES = {
    T_ARC: "円弧", T_TEXT: "文字", T_GROUP: "グループ", 13: "角丸長方形", 14: "画像",
    T_SYMBOL: "シンボル", T_DIMENSION: "寸法", T_PLUGIN: "プラグインオブジェクト",
    T_VIEWPORT: "ビューポート",
}

# 日本語版 Vectorworks 2023 では「一般」(実機で確認)。英語版は None
NONE_NAMES = ("一般", "なし", "None")
DIMENSION_NAMES = ("寸法", "Dimension")

VERTEX_KINDS = {0: "corner", 1: "bezier", 2: "cubic", 3: "arc", 4: "arc"}


class Exporter:
    def __init__(self, vs):
        self.vs = vs
        self.warnings = {}  # 同じ注意はまとめて数える
        self.counts = {}
        self.unsupported = {}
        self.resources = {"lineTypes": {}, "hatches": {}, "tiles": {}, "gradients": {}, "imageFills": {}}
        self.class_ids = {}
        self.mm_per_unit = 1.0
        self.seen_ids = set()

    # ---------- 共通 ----------
    def warn(self, msg):
        self.warnings[msg] = self.warnings.get(msg, 0) + 1

    def warning_list(self):
        return [m if n == 1 else "{}(×{})".format(m, n) for m, n in self.warnings.items()]

    def safe(self, label, fn, default=None):
        """1つの項目が読めなくても全体は止めない。読めなかったことは記録する。"""
        try:
            return fn()
        except Exception as e:  # noqa: BLE001
            self.warn("{} が読めませんでした: {}".format(label, e))
            return default

    def mm(self, v):
        return round(v * self.mm_per_unit, 6)

    def pt(self, p):
        return [self.mm(p[0]), self.mm(p[1])]

    @staticmethod
    def color(rgb):
        # Vectorworks は 0〜65535、VectorStation は 0〜255
        return [int(round(c / 257.0)) for c in rgb[:3]]

    @staticmethod
    def mils_to_mm(mils):
        return round(mils * 0.0254, 4)

    def unique_id(self, base):
        i, cand = 1, base
        while cand in self.seen_ids:
            i += 1
            cand = "{}-{}".format(base, i)
        self.seen_ids.add(cand)
        return cand

    # ---------- リソース ----------
    def resource_ref(self, kind, name):
        table = self.resources[kind]
        if name not in table:
            prefix = {"lineTypes": "lt", "hatches": "hat", "tiles": "tile", "gradients": "grad", "imageFills": "img"}[kind]
            table[name] = {"id": self.unique_id("{}-vw-{}".format(prefix, len(table) + 1)), "name": name, "source": "vectorworks"}
        return table[name]["id"]

    def line_type_from_lsn(self, lsn):
        """GetLSN の値: 負の値は線種リソースの名前の番号、0 は線なし、それ以外は実線。
        (0=線なし、2=実線 は Vectorworks 2023 の実機で値を確認)"""
        vs = self.vs
        if lsn is None:
            return {"kind": "solid"}
        if lsn < 0:
            name = vs.Index2Name(-lsn)
            if name:
                return {"kind": "resource", "resource": self.resource_ref("lineTypes", name)}
        if lsn == 0:
            return {"kind": "none"}
        return {"kind": "solid"}

    def fill_from_fpat(self, fpat):
        """GetFPat の値: 0=なし、1=単色、2以上=模様、負の値=ハッチング等のリソース。"""
        vs = self.vs
        if fpat is None:
            return None
        if fpat == 0:
            return {"kind": "none"}
        if fpat == 1:
            return {"kind": "solid"}
        if fpat > 1:
            return {"kind": "pattern", "resource": "pat-vw-{}".format(fpat)}
        name = vs.Index2Name(-fpat)
        h = vs.GetObject(name) if name else None
        t = vs.GetTypeN(h) if h else None
        kind = {T_HATCH: ("hatch", "hatches"), T_TILE: ("tile", "tiles"),
                T_GRADIENT: ("gradient", "gradients"), T_IMAGEFILL: ("image", "imageFills")}.get(t)
        if kind is None:
            self.warn("面の種類がわかりません: {}".format(name))
            return {"kind": "solid"}
        if kind[0] in ("gradient", "image"):
            self.warn("グラデーション・イメージの面はまだ対応していません({})。単色として書き出しました".format(name))
            self.resource_ref(kind[1], name)
            return {"kind": "solid"}
        return {"kind": kind[0], "resource": self.resource_ref(kind[1], name)}

    # ---------- クラス ----------
    def export_classes(self):
        vs = self.vs
        classes = []
        n = vs.ClassNum()
        for i in range(1, n + 1):
            name = vs.ClassList(i)
            if not name:
                continue
            if name in NONE_NAMES:
                cid, builtin = self.unique_id("cls-none"), "none"
            elif name in DIMENSION_NAMES:
                cid, builtin = self.unique_id("cls-dimension"), "dimension"
            else:
                cid, builtin = self.unique_id("cls-vw-{}".format(i)), None
            self.class_ids[name] = cid
            vis = self.safe("クラス「{}」の表示".format(name), lambda: vs.GetCVis(name), 0)
            attrs = {}
            lw = self.safe("クラス「{}」の線の太さ".format(name), lambda: vs.GetClLW(name))
            if lw is not None:
                attrs["lineWeight"] = self.mils_to_mm(lw)
            pen = self.safe("クラス「{}」の線の色".format(name), lambda: vs.GetClPenFore(name))
            if pen:
                attrs["penColor"] = self.color(pen)
            fillc = self.safe("クラス「{}」の面の色".format(name), lambda: vs.GetClFillFore(name))
            if fillc:
                attrs["fillColor"] = self.color(fillc)
            lsn = self.safe("クラス「{}」の線種".format(name), lambda: vs.GetClLSN(name))
            if lsn is not None:
                attrs["lineType"] = self.line_type_from_lsn(lsn)
            fill = None
            vf = self.safe("クラス「{}」のハッチング".format(name), lambda: vs.GetClVectorFill(name))
            if vf and vf[0] and vf[1]:
                fill = self.fill_from_fpat(-vs.Name2Index(vf[1]))
            else:
                fpat = self.safe("クラス「{}」の面".format(name), lambda: vs.GetClFPat(name))
                fill = self.fill_from_fpat(fpat)
            if fill:
                attrs["fill"] = fill
            op = self.safe("クラス「{}」の不透明度".format(name), lambda: vs.GetClOpacityN(name))
            if op:
                attrs["fillOpacity"], attrs["penOpacity"] = int(op[0]), int(op[1])
            c = {
                "id": cid,
                "name": name,
                "visibility": {0: "visible", 2: "grayed", -1: "hidden"}.get(vis, "visible"),
                "attributes": attrs,
                "useAtCreation": bool(self.safe("クラス「{}」の作成時設定".format(name), lambda: vs.GetClUseGraphic(name), False)),
            }
            if builtin:
                c["builtin"] = builtin
            classes.append(c)
        # 必須クラスがない(名前が変えられている等)場合は補う
        for builtin, cid, label in (("none", "cls-none", "一般"), ("dimension", "cls-dimension", "寸法")):
            if not any(c.get("builtin") == builtin for c in classes):
                self.warn("必須クラス「{}」が見つからなかったので追加しました".format(label))
                classes.insert(0, {"id": self.unique_id(cid), "name": label, "visibility": "visible",
                                   "attributes": {}, "useAtCreation": False, "builtin": builtin})
        return classes

    def class_id_of(self, h):
        vs = self.vs
        name = self.safe("図形のクラス", lambda: vs.GetClass(h), None)
        if name in self.class_ids:
            return self.class_ids[name]
        for k in NONE_NAMES:
            if k in self.class_ids:
                return self.class_ids[k]
        return "cls-none"

    # ---------- 図形の属性 ----------
    def object_attributes(self, h):
        vs = self.vs
        a = {}

        def attr(key, by_class_fn, value_fn):
            by = self.safe("属性({})のクラス設定".format(key), by_class_fn, False)
            if by:
                a[key] = {"byClass": True}
            else:
                v = self.safe("属性({})".format(key), value_fn)
                if v is not None:
                    a[key] = {"value": v}

        attr("lineWeight", lambda: vs.IsLWByClass(h), lambda: self.mils_to_mm(vs.GetLW(h)))
        attr("penColor", lambda: vs.IsPenColorByClass(h), lambda: self.color(vs.GetPenFore(h)))
        attr("fillColor", lambda: vs.IsFillColorByClass(h), lambda: self.color(vs.GetFillFore(h)))
        attr("lineType", lambda: vs.IsLSByClass(h), lambda: self.line_type_from_lsn(vs.GetLSN(h)))
        attr("fill", lambda: vs.IsFPatByClass(h), lambda: self.fill_from_fpat(vs.GetFPat(h)))
        byc = self.safe("不透明度のクラス設定", lambda: vs.GetOpacityByClassN(h), (False, False))
        op = self.safe("不透明度", lambda: vs.GetOpacityN(h), None)
        if byc and byc[0]:
            a["penOpacity"] = {"byClass": True}
        elif op:
            a["penOpacity"] = {"value": int(op[1])}
        if byc and byc[1]:
            a["fillOpacity"] = {"byClass": True}
        elif op:
            a["fillOpacity"] = {"value": int(op[2])}
        return a

    # ---------- 図形 ----------
    def object_id(self, h):
        uuid = self.safe("図形のID", lambda: self.vs.GetObjectUuid(h), "")
        return self.unique_id("obj-vw-{}".format(uuid.strip("{}").replace("-", "").lower()) if uuid else "obj-vw")

    def unsupported_obj(self, h, t, label):
        vs = self.vs
        self.unsupported[label] = self.unsupported.get(label, 0) + 1
        bbox = self.safe("図形の範囲", lambda: vs.GetBBox(h), None)
        geo = {"vwType": t}
        if bbox:
            geo["bbox"] = [self.pt(bbox[0]), self.pt(bbox[1])]
        if t == T_ARC:
            arc = self.safe("円弧", lambda: vs.GetArc(h), None)
            if arc:
                geo["startAngle"], geo["sweepAngle"] = arc[0], arc[1]
        if t == T_GROUP:
            geo["children"] = self.export_children(vs.FInGroup(h))
        return "vw-" + str(t), geo

    def export_object(self, h):
        vs = self.vs
        t = vs.GetTypeN(h)
        kind, geo = None, None
        if t == T_LINE:
            p1, p2 = vs.GetSegPt1(h), vs.GetSegPt2(h)
            kind, geo = "line", {"from": self.pt(p1), "to": self.pt(p2)}
        elif t in (T_RECT, T_OVAL):
            ang = self.safe("角度", lambda: vs.HAngle(h), 0.0) or 0.0
            if abs(ang) > 1e-9:
                kind, geo = self.unsupported_obj(h, t, "回転した長方形・楕円")
                geo["rotation"] = ang
            else:
                p1, p2 = vs.GetBBox(h)
                c = [(p1[0] + p2[0]) / 2.0, (p1[1] + p2[1]) / 2.0]
                w, hh = abs(p2[0] - p1[0]), abs(p1[1] - p2[1])
                if t == T_RECT:
                    kind, geo = "rect", {"center": self.pt(c), "width": self.mm(w), "height": self.mm(hh), "rotation": 0.0}
                else:
                    kind, geo = "ellipse", {"center": self.pt(c), "rx": self.mm(w / 2.0), "ry": self.mm(hh / 2.0), "rotation": 0.0}
        elif t in (T_POLY, T_POLYLINE):
            verts = []
            for i in range(1, vs.GetVertNum(h) + 1):
                p, vt, r = vs.GetPolylineVertex(h, i)
                v = {"p": self.pt(p)}
                k = VERTEX_KINDS.get(vt, "corner")
                if k == "arc":
                    v["vertex"] = {"kind": "arc", "radius": self.mm(r)} if r else {"kind": "arc"}
                elif k != "corner":
                    v["vertex"] = {"kind": k}
                verts.append(v)
            closed = bool(self.safe("閉じているか", lambda: vs.IsPolyClosed(h), False))
            kind, geo = "polyline", {"vertices": verts, "closed": closed}
        else:
            kind, geo = self.unsupported_obj(h, t, TYPE_NAMES.get(t, "種類{}".format(t)))

        self.counts[kind] = self.counts.get(kind, 0) + 1
        obj = {
            "id": self.object_id(h),
            "type": kind,
            "classId": self.class_id_of(h),
            "geometry": geo,
            "attributes": self.object_attributes(h),
        }
        return obj

    def export_children(self, h):
        out = []
        while h:
            obj = self.safe("図形", lambda: self.export_object(h))
            if obj:
                out.append(obj)
            h = self.vs.NextObj(h)
        return out

    # ---------- レイヤ ----------
    def export_layers(self):
        vs = self.vs
        design, sheets = [], []
        h = vs.FLayer()
        n = 0
        while h:
            n += 1
            name = vs.GetLName(h)
            ltype = self.safe("レイヤ「{}」の種類".format(name), lambda: vs.GetObjectVariableInt(h, 154), 1)
            if ltype == 2:
                sheets.append({"id": self.unique_id("sht-vw-{}".format(n)), "name": name, "source": "vectorworks",
                               "note": "シートレイヤは第1版では名前だけ書き出す"})
            else:
                vis = self.safe("レイヤ「{}」の表示".format(name), lambda: vs.GetLVis(h), 0)
                elev = self.safe("レイヤ「{}」の高さ".format(name), lambda: vs.GetLayerElevation(h), (0.0, 0.0))
                scale = self.safe("レイヤ「{}」の縮尺".format(name), lambda: vs.GetLScale(h), 1.0) or 1.0
                if scale < 1:
                    self.warn("レイヤ「{}」の縮尺 1:{} は VectorStation ではまだ扱えないので 1:1 にしました".format(name, scale))
                    scale = 1.0
                design.append({
                    "id": self.unique_id("lay-vw-{}".format(n)),
                    "name": name,
                    "scale": scale,
                    "visibility": {0: "visible", 2: "grayed", -1: "hidden"}.get(vis, "visible"),
                    "locked": False,
                    "elevation": self.mm(elev[0]) if elev else 0.0,
                    "objects": self.export_children(vs.FInLayer(h)),
                })
            h = vs.NextLayer(h)
        return design, sheets

    # ---------- 全体 ----------
    def export(self):
        vs = self.vs
        units = self.safe("単位", lambda: vs.GetUnits(), None)
        if units and units[3]:
            self.mm_per_unit = 25.4 / units[3]
        classes = self.export_classes()
        design, sheets = self.export_layers()
        if not design:
            design = [{"id": "lay-1", "name": "デザインレイヤ-1", "scale": 50.0, "visibility": "visible",
                       "locked": False, "elevation": 0.0, "objects": []}]
        active_layer = self.safe("アクティブなレイヤ", lambda: vs.GetLName(vs.ActLayer()), None)
        active_class = self.safe("アクティブなクラス", lambda: vs.ActiveClass(), None)
        state = {}
        for l in design:
            if l["name"] == active_layer:
                state["activeLayer"] = l["id"]
        if active_class in self.class_ids:
            state["activeClass"] = self.class_ids[active_class]
        resources = {k: list(v.values()) for k, v in self.resources.items() if v}
        doc = {
            "settings": {"units": "mm"},
            "classes": classes,
            "designLayers": design,
            "sheetLayers": sheets,
            "resources": resources,
            "state": state,
            "exportedFrom": {
                "app": "Vectorworks",
                "exporter": "vs_export.py " + EXPORTER_VERSION,
                "file": self.safe("ファイル名", lambda: vs.GetFPathName(), ""),
                "warnings": self.warning_list(),
                "unsupported": self.unsupported,
            },
        }
        return {"formatVersion": FORMAT_VERSION, "app": "VectorStation", "document": doc}

    def report(self, path):
        lines = ["VectorStation 形式で書き出しました。", path, ""]
        lines.append("書き出した図形: " + (", ".join("{} {}個".format(k, v) for k, v in sorted(self.counts.items()) if not k.startswith("vw-")) or "なし"))
        if self.unsupported:
            lines.append("まだ対応していない図形(中身の一部だけ保持): " + ", ".join("{} {}個".format(k, v) for k, v in self.unsupported.items()))
        if self.warnings:
            total = sum(self.warnings.values())
            lines.append("注意 {}件、{}種類(ファイル内の exportedFrom.warnings に全件)".format(total, len(self.warnings)))
            lines.extend("・" + w for w in self.warning_list()[:5])
        return "\n".join(lines)


def output_path(vwx_path):
    base, _ = os.path.splitext(vwx_path)
    return base + ".vws"


def run(vs):
    ex = Exporter(vs)
    data = ex.export()
    src = vs.GetFPathName()
    if not src:
        vs.AlrtDialog("先に図面を保存してください(書き出し先を図面と同じフォルダにするため)。")
        return None
    path = output_path(src)
    # 図形が多い図面は、ファイルを小さくするため改行・字下げを省く
    total = sum(ex.counts.values())
    with open(path, "w", encoding="utf-8") as f:
        if total > 5000:
            json.dump(data, f, ensure_ascii=False, separators=(",", ":"))
        else:
            json.dump(data, f, ensure_ascii=False, indent=2)
        f.write("\n")
    vs.AlrtDialog(ex.report(path))
    return path


# Vectorworks の中で実行されたときだけ動く(vs モジュールは Vectorworks の中にしかない)
try:
    import vs as _vs
except ImportError:
    _vs = None
if _vs is not None:
    run(_vs)
