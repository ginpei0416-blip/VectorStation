# -*- coding: utf-8 -*-
"""書き出しスクリプトの確認(Vectorworks なしで、代わりの vs モジュールを使う)。
使い方: python3 tools/vw-export/tests/test_export.py <vs コマンドのパス>
書き出したファイルを VectorStation(vs info / vs resolve)で実際に開いて確かめる。"""

import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
sys.path.insert(0, HERE)

import vs_export  # noqa: E402
from fake_vs import FakeVS, Layer, Obj  # noqa: E402

VS_BIN = sys.argv[1] if len(sys.argv) > 1 else "vs"
failures = []


def check(cond, msg):
    print(("OK  " if cond else "NG  ") + msg)
    if not cond:
        failures.append(msg)


def build():
    # 名前リスト: 5=一点鎖線(線種), 7=コンクリート(ハッチング), 8=木目(タイル), 9=夕焼け(グラデーション)
    names = {5: ("一点鎖線", 96), 7: ("コンクリート", 66), 8: ("木目", 108), 9: ("夕焼け", 120)}
    classes = {
        "なし": {},
        "寸法": {},
        "00_基準線": {"lw": 7, "pen": (65535, 0, 0), "lsn": -5, "use": True},  # 7mil ≒ 0.18mm
        "01_壁-既存": {"lw": 14, "vf": "コンクリート", "vis": 2},
        "03_家具": {"lw": 7, "op": (60, 100), "vis": -1},
    }
    grid = Obj(2, "00_基準線", p1=(0.0, -1000.0), p2=(0.0, 9000.0),
               attrs={"byclass": ("lw", "pen", "ls", "fill", "fpat", "penop", "fillop")})
    wall = Obj(21, "01_壁-既存", verts=[((0.0, 0.0), 0, 0.0), ((6000.0, 0.0), 0, 0.0), ((6000.0, 8000.0), 3, 300.0)],
               closed=True, attrs={"lw": 14, "fpat": -7, "op": (100, 80)})
    table = Obj(3, "03_家具", bbox=((2100.0, 4450.0), (3900.0, 3550.0)),
                attrs={"fill": (59110, 56540, 51400), "op": (100, 60), "fpat": -8})
    column = Obj(4, "01_壁-既存", bbox=((-300.0, 300.0), (300.0, -300.0)), attrs={"fpat": 0})
    rotated = Obj(3, "03_家具", bbox=((0.0, 10.0), (10.0, 0.0)), angle=30.0)
    sky = Obj(3, "なし", bbox=((0.0, 10.0), (10.0, 0.0)), attrs={"fpat": -9})
    text = Obj(10, "なし", bbox=((0.0, 10.0), (50.0, 0.0)))
    group = Obj(11, "なし", bbox=((0.0, 10.0), (10.0, 0.0)), children=[Obj(2, "なし", p1=(0.0, 0.0), p2=(1.0, 1.0))])
    l1 = Layer("1F平面", 50.0, 0, objs=[grid, wall, table, column, rotated, sky, text, group])
    l2 = Layer("詳細", 10.0, 2)
    sheet = Layer("A-01 平面図", 1.0, 0, ltype=2)
    return FakeVS(classes, [l1, l2, sheet], names=names, active_layer=l1, active_class="00_基準線")


def main():
    fake = build()
    with tempfile.TemporaryDirectory() as d:
        fake.path = os.path.join(d, "テスト.vwx")
        path = vs_export.run(fake)
        check(path == os.path.join(d, "テスト.vws"), "図面と同じフォルダに「図面名.vws」で書き出す")
        data = json.load(open(path, encoding="utf-8"))
        doc = data["document"]
        check(data["formatVersion"] == 1, "形式1で書き出す")
        names = [c["name"] for c in doc["classes"]]
        check(names == ["なし", "寸法", "00_基準線", "01_壁-既存", "03_家具"], "クラスを並び順どおりに書き出す")
        cls = {c["name"]: c for c in doc["classes"]}
        check(cls["00_基準線"]["useAtCreation"] is True, "作成時にクラスの属性を使う設定を書き出す")
        check(cls["00_基準線"]["attributes"]["lineWeight"] == 0.1778, "線の太さを mil から mm に変換する(7mil=0.1778mm)")
        check(cls["00_基準線"]["attributes"]["penColor"] == [255, 0, 0], "色を 0〜255 に変換する")
        check(cls["00_基準線"]["attributes"]["lineType"]["kind"] == "resource", "線種リソースの参照を書き出す")
        check(cls["01_壁-既存"]["attributes"]["fill"]["kind"] == "hatch", "クラスのハッチングを書き出す")
        check(cls["01_壁-既存"]["visibility"] == "grayed" and cls["03_家具"]["visibility"] == "hidden", "クラスのグレー表示・非表示")
        check(cls["03_家具"]["attributes"]["fillOpacity"] == 60, "クラスの面の不透明度")
        layers = doc["designLayers"]
        check([l["name"] for l in layers] == ["1F平面", "詳細"], "デザインレイヤだけを designLayers に入れる")
        check(layers[1]["scale"] == 10.0 and layers[1]["visibility"] == "grayed", "レイヤごとの縮尺と表示状態")
        check(doc["sheetLayers"][0]["name"] == "A-01 平面図", "シートレイヤは名前だけ書き出す")
        objs = layers[0]["objects"]
        types = [o["type"] for o in objs]
        check(types[:4] == ["line", "polyline", "rect", "ellipse"], "直線・ポリライン・長方形・楕円を書き出す")
        check(objs[0]["attributes"]["lineWeight"] == {"byClass": True}, "クラスに従う属性は byClass で書き出す")
        check(objs[1]["geometry"]["vertices"][2]["vertex"] == {"kind": "arc", "radius": 300.0}, "円弧頂点と半径を書き出す")
        check(objs[2]["geometry"] == {"center": [3000.0, 4000.0], "width": 1800.0, "height": 900.0, "rotation": 0.0}, "長方形の中心・幅・高さ")
        check(objs[2]["attributes"]["fill"]["value"]["kind"] == "tile", "タイルの面を書き出す")
        check(objs[2]["attributes"]["fillOpacity"] == {"value": 60}, "図形の面の不透明度")
        check(objs[3]["geometry"]["rx"] == 300.0, "円の半径")
        check(objs[4]["type"].startswith("vw-") and objs[4]["geometry"]["rotation"] == 30.0, "回転した長方形は未対応として保持する")
        check(objs[5]["attributes"]["fill"]["value"]["kind"] == "solid", "グラデーションは単色として書き出す(未対応)")
        check(objs[7]["geometry"]["children"][0]["type"] == "line", "グループの中身も保持する")
        check(doc["state"]["activeClass"] == cls["00_基準線"]["id"], "アクティブなクラス")
        rep = fake.dialogs[-1]
        check("回転した長方形・楕円 1個" in rep and "文字 1個" in rep, "未対応の図形を種類ごとに報告する")
        ids = [o["id"] for o in objs]
        check(len(set(ids)) == len(ids), "図形のIDが重複しない")

        # VectorStation で実際に開けるか
        r = subprocess.run([VS_BIN, "info", path], capture_output=True, text=True, encoding="utf-8")
        check(r.returncode == 0, "VectorStation で開ける: " + (r.stderr.strip() or "OK"))
        check("1F平面 1:50" in r.stdout and "00_基準線" in r.stdout, "vs info に内容が出る")
        r = subprocess.run([VS_BIN, "resolve", path, objs[0]["id"]], capture_output=True, text=True, encoding="utf-8")
        res = json.loads(r.stdout) if r.returncode == 0 else {}
        check(res.get("penColor") == [255, 0, 0] and res.get("lineWeight") == 0.1778, "通り芯の最終的な見た目がクラスどおりになる")

    print()
    if failures:
        print("失敗 {}件".format(len(failures)))
        sys.exit(1)
    print("すべて通過")


if __name__ == "__main__":
    main()
