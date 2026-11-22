# -*- coding: utf-8 -*-
"""Vectorworks の vs モジュールの代わり(テスト用)。
公開リファレンスに書かれた関数の形(引数と戻り値)だけを真似て、メモリ上の図面を返す。
本物の Vectorworks での確認は、作成者の PC で別に行う。"""

UNITS_MM = 25.4  # 1インチあたり 25.4 単位 = mm


class Obj:
    def __init__(self, t, cls="なし", **kw):
        self.t, self.cls, self.kw = t, cls, kw
        self.next = None
        self.children = kw.get("children", [])
        self.uuid = kw.get("uuid", "{%08X-0000-0000-0000-000000000000}" % id(self))


class Layer:
    def __init__(self, name, scale=50.0, vis=0, ltype=1, objs=()):
        self.name, self.scale, self.vis, self.ltype = name, scale, vis, ltype
        self.objs = list(objs)
        self.next = None


def link(items):
    for a, b in zip(items, items[1:]):
        a.next = b
    return items[0] if items else None


class FakeVS:
    def __init__(self, classes, layers, names=None, path="C:/案件/テスト.vwx", active_layer=None, active_class="なし"):
        # classes: {名前: dict(lw, pen, fill, lsn, fpat, op=(fill, pen), vis, use, vf)}
        self.classes = classes
        self.layers = layers
        self.names = names or {}  # 名前リストの番号 -> (名前, 種類番号)
        self.path = path
        self.active_layer = active_layer or layers[0]
        self.active_class = active_class
        link(layers)
        for l in layers:
            link(l.objs)
            for o in l.objs:
                if o.children:
                    link(o.children)
        self.dialogs = []

    # 書類
    def GetUnits(self): return (1, 0, 0, UNITS_MM, "mm", "m²")
    def GetFPathName(self): return self.path
    def AlrtDialog(self, msg): self.dialogs.append(msg)
    def ActLayer(self): return self.active_layer
    def ActiveClass(self): return self.active_class
    def Index2Name(self, i): return self.names.get(i, ("", 0))[0]
    def Name2Index(self, name):
        for i, (n, _) in self.names.items():
            if n == name:
                return i
        return 0
    def GetObject(self, name):
        for i, (n, t) in self.names.items():
            if n == name:
                return ("res", t)
        return None

    # クラス
    def ClassNum(self): return len(self.classes)
    def ClassList(self, i): return list(self.classes)[i - 1]
    def _c(self, n): return self.classes[n]
    def GetCVis(self, n): return self._c(n).get("vis", 0)
    def GetClLW(self, n): return self._c(n).get("lw", 5)
    def GetClPenFore(self, n): return self._c(n).get("pen", (0, 0, 0))
    def GetClFillFore(self, n): return self._c(n).get("fill", (65535, 65535, 65535))
    def GetClLSN(self, n): return self._c(n).get("lsn", 2)
    def GetClFPat(self, n): return self._c(n).get("fpat", 1)
    def GetClVectorFill(self, n):
        vf = self._c(n).get("vf")
        return (True, vf) if vf else (False, "")
    def GetClOpacityN(self, n): return self._c(n).get("op", (100, 100))
    def GetClUseGraphic(self, n): return self._c(n).get("use", False)

    # レイヤ
    def FLayer(self): return self.layers[0]
    def NextLayer(self, h): return h.next
    def GetLName(self, h): return h.name
    def GetObjectVariableInt(self, h, sel): return h.ltype if sel == 154 else 0
    def GetLVis(self, h): return h.vis
    def GetLayerElevation(self, h): return (0.0, 0.0)
    def GetLScale(self, h): return h.scale
    def FInLayer(self, h): return h.objs[0] if h.objs else None
    def FInGroup(self, h): return h.children[0] if h.children else None
    def NextObj(self, h): return h.next

    # 図形
    def GetTypeN(self, h):
        if isinstance(h, tuple):
            return h[1]
        return h.t
    def GetClass(self, h): return h.cls
    def GetObjectUuid(self, h): return h.uuid
    def GetSegPt1(self, h): return h.kw["p1"]
    def GetSegPt2(self, h): return h.kw["p2"]
    def GetBBox(self, h): return h.kw["bbox"]
    def HAngle(self, h): return h.kw.get("angle", 0.0)
    def GetVertNum(self, h): return len(h.kw["verts"])
    def GetPolylineVertex(self, h, i): return h.kw["verts"][i - 1]
    def IsPolyClosed(self, h): return h.kw.get("closed", False)
    def GetArc(self, h): return h.kw.get("arc", (0.0, 90.0))
    def _a(self, h, k, d): return h.kw.get("attrs", {}).get(k, d)
    def IsLWByClass(self, h): return "lw" in self._a(h, "byclass", ())
    def IsPenColorByClass(self, h): return "pen" in self._a(h, "byclass", ())
    def IsFillColorByClass(self, h): return "fill" in self._a(h, "byclass", ())
    def IsLSByClass(self, h): return "ls" in self._a(h, "byclass", ())
    def IsFPatByClass(self, h): return "fpat" in self._a(h, "byclass", ())
    def GetOpacityByClassN(self, h):
        b = self._a(h, "byclass", ())
        return ("penop" in b, "fillop" in b)
    def GetLW(self, h): return self._a(h, "lw", 5)
    def GetPenFore(self, h): return self._a(h, "pen", (0, 0, 0))
    def GetFillFore(self, h): return self._a(h, "fill", (65535, 65535, 65535))
    def GetLSN(self, h): return self._a(h, "lsn", 2)
    def GetFPat(self, h): return self._a(h, "fpat", 1)
    def GetOpacityN(self, h):
        op = self._a(h, "op", (100, 100))
        return (True, op[0], op[1])
