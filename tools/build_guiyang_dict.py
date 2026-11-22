#!/usr/bin/env python3
"""Build `public/data/guiyang.txt.gz` from the corpus sheet (`dialect17277/audio16k_pinyin.json`).

One line per character: `汉字<TAB>拼音`. The sheet has 1000 readings for 994 characters, and
29 多音/文白 entries carry two readings in one cell ("去 ke4 qi4") — the first one is the
dialect reading and is what we keep, so the dictionary has a single reading per character and
both engines need no special rule. 闩 has no reading at all and is dropped.

Usage:
  python3 tools/build_guiyang_dict.py [sheet.json] [-o public/data/guiyang.txt.gz]
"""
import argparse
import gzip
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("sheet", nargs="?", default=str(ROOT.parent / "frft-vocal-synth/dialect17277/audio16k_pinyin.json"))
    ap.add_argument("-o", "--out", default=str(ROOT / "public/data/guiyang.txt.gz"))
    a = ap.parse_args()
    recs = json.loads(pathlib.Path(a.sheet).read_text(encoding="utf-8"))
    seen: dict[str, str] = {}
    for r in recs:
        p = r["拼音"].split()
        if not p or r["单字"] in seen:
            continue
        seen[r["单字"]] = p[0]
    text = "".join(f"{k}\t{seen[k]}\n" for k in sorted(seen))
    with gzip.GzipFile(a.out, "wb", mtime=0) as f:
        f.write(text.encode("utf-8"))
    print(f"{len(seen)} 字 → {a.out}")


if __name__ == "__main__":
    main()
