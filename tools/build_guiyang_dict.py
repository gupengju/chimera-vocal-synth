#!/usr/bin/env python3
"""Build the Guiyang dictionaries from the two dialect corpora + 施文同音字表.

Sources of truth:

  * `dialect17277/audio16k_danzi.json` — 1000 单字（编号/单字/注例/读音）
  * `dialect17277/audio16k_cihui.json` — 1200 词汇（编号/词汇/注例/读音/说法）
  * `dialect17277/guiyang_tongyinzi_shiwen.txt` — 施文《贵阳方言音系调查研究》附录二
    同音字表（`extract_tongyinzi.py` 从 PDF 抽的，韵母|声母|调类|字），补单字覆盖

Every reading is converted to the project's 贵阳记法 by `dialect17277/build_pinyin.py`.
词条按**实际说法**记，不按问卷词头：`说法` 里的每个词形配它自己那段读音，所以 星星 那一格
既有 `星宿` 也有 `星星`，`肏` 那一格记的是 `日`（读 zɿ21），文字与读音对不上时以说法为准。
两种记号按“文字侧”处理：`=`（借音字，如 `哪港=`）去掉标记后照字记；`#n`（本字未定）没有
文字可记，单独落到 `guiyang_unresolved.csv` 等补。

同音字表是**后补源**：语料读音先入表、天然占住默认位，书上的另一读只作追加行；书里 ȵ 母
按录音人的实际读法并入 l（你 li3、南 lan2），调类对位、调值不搬（见 build_pinyin.py
的 SHI_* 表）。同形同音的重复不再写第二行。

产出三个文件，主表是 `词形<TAB>读音`：

  * `public/data/guiyang.txt` — **全部**读音，一个读音一行，语料顺序（同形可多行，单字在前、
    词汇在后，同音字表补在最后），首行即默认读音。多音/多说法一律平铺，不分白/文，也不做
    义项分类。Rust / C++ 两端装载「先出现者胜」，多行不互相覆盖，词条按最长匹配生效。
    纯文本直接改；`bun run data`（`bun run dev` / `bun run build` 都会先跑它）压出同目录的
    `guiyang.txt.gz` 供页面装载。
  * `public/data/guiyang_readings.csv` — 逐条溯源：词形/读音/IPA/编号/来源/词汇/注例/默认。
  * `public/data/guiyang_unresolved.csv` — 只有 `#n`（本字未定）的 7 条，等补本字后再进表。

Usage:
  python3 tools/build_guiyang_dict.py [danzi.json] [--words audio16k_cihui.json]
                                      [--tongyin guiyang_tongyinzi_shiwen.txt]
"""
import argparse
import csv
import pathlib
import sys
from collections import Counter

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent
CORPUS_DIR = ROOT.parent / "frft-vocal-synth/dialect17277"
sys.path.insert(0, str(CORPUS_DIR))
import build_pinyin as bp  # noqa: E402  (同目录的记法转换表，单一来源)

CSV_FIELDS = ["词形", "读音", "IPA", "编号", "来源", "词汇", "注例", "默认"]


def normalize_form(token: str) -> tuple[str, bool]:
    """说法 token → 记录用的词形；`[不要]` 是合音写法，返回 (词形, 是否合音)。"""
    t = token.replace("=", "")
    fused = t.startswith("[") and t.endswith("]")
    return (t[1:-1] if fused else t), fused


def char_entries(records):
    """单字语料 → (词形, 记法, IPA, 记录)。"""
    for r in records:
        ipa = r.get("读音", "").strip().split()
        for s in ipa:
            yield r["单字"], bp.syllable_to_notation(s), s, r


def word_entries(records, unresolved: list):
    """词汇语料 → (词形, 记法, IPA, 记录)。"""

    def drop(r, why):
        unresolved.append({"编号": r.get("编号", ""), "词汇": r.get("词汇", ""),
                           "说法": r.get("说法", ""), "读音": r.get("读音", ""), "原因": why})

    for r in records:
        raw = r.get("读音", "").strip()
        if not raw:
            continue
        says = r.get("说法", "").split()
        groups = raw.split()
        if len(says) != len(groups):
            # 组内误插空格的（0646 堂兄弟、0647 表兄弟：`…ɕioŋ55t i13`）整行当一条读音
            if len(says) == 1:
                groups = [raw]
            else:
                drop(r, f"说法 {len(says)} 组 vs 读音 {len(groups)} 组")
                continue
        for say, g in zip(says, groups):
            form, fused = normalize_form(say)
            if not form:
                drop(r, "说法为空")
                continue
            if "#" in form:
                drop(r, f"本字未定（{say}）")
                continue
            syls = bp.split_ipa_group(g)
            if len(syls) != len(form) and not (fused and len(syls) == 1):
                drop(r, f"{form} 有 {len(form)} 字、{len(syls)} 音节")
                continue
            yield form, " ".join(bp.syllable_to_notation(s) for s in syls), g, r


def tongyin_entries(path, unresolved: list):
    """施文同音字表 txt → (词形, 记法, IPA, 记录)。

    行格式 `韵母|声母|调类|字 字 字`，`#` 开头是注释。一格里每个字都拿同一读音；
    映射不上直接进 unresolved 并让主流程报错退出，不猜。
    """
    if not path.exists():
        return
    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        parts = s.split("|")
        if len(parts) != 4:
            unresolved.append({"编号": f"TY{lineno:04d}", "词汇": "", "说法": s,
                               "读音": "", "原因": f"同音字表第 {lineno} 行不是 4 列"})
            continue
        fin, ini, tone, chars = parts[0].strip(), parts[1].strip(), parts[2].strip(), parts[3].split()
        try:
            note = bp.shi_syllable_to_notation(fin, ini, tone)
        except KeyError as e:
            unresolved.append({"编号": f"TY{lineno:04d}", "词汇": "", "说法": s,
                               "读音": "", "原因": f"映射不上：{e}"})
            continue
        ipa = bp.shi_syllable_ipa(fin, ini, tone)
        rec = {"编号": f"TY{lineno:04d}",
               "注例": f"[{fin}] {ini or '∅'} {tone}（书调值 {bp.SHI_TONE_VALUE[tone]}）"}
        for c in chars:
            yield c, note, ipa, rec


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("corpus", nargs="?", default=str(CORPUS_DIR / "audio16k_danzi.json"))
    ap.add_argument("--words", default=str(CORPUS_DIR / "audio16k_cihui.json"))
    ap.add_argument("--tongyin", default=str(CORPUS_DIR / "guiyang_tongyinzi_shiwen.txt"))
    ap.add_argument("-o", "--out", default=str(ROOT / "public/data/guiyang.txt"))
    ap.add_argument("--csv", default=str(ROOT / "public/data/guiyang_readings.csv"))
    ap.add_argument("--unresolved", default=str(ROOT / "public/data/guiyang_unresolved.csv"))
    ap.add_argument("--books", default=str(CORPUS_DIR / "books4_guiyang.tsv"),
                    help="《贵阳方言》书稿词表（词形<TAB>读音）；存在则整表前置，首行即默认")
    a = ap.parse_args()

    unresolved: list[dict] = []
    multi: list[tuple[str, str]] = []            # 语料顺序；单字在前
    seen: dict[str, set[str]] = {}
    default: dict[str, str] = {}
    csv_rows: list[dict] = []

    def add(form, note, ipa, rec, source, word="", skip_dupe=False):
        is_default = form not in default
        if is_default:
            default[form] = note
        bucket = seen.setdefault(form, set())
        is_new = note not in bucket
        bucket.add(note)
        if is_new:
            multi.append((form, note))
        elif skip_dupe:
            return                                  # 同形同音：书上的重复不再记一行
        csv_rows.append({"词形": form, "读音": note, "IPA": ipa,
                         "编号": rec.get("编号", ""), "来源": source,
                         "词汇": word, "注例": rec.get("注例", ""),
                         "默认": "1" if is_default else "0"})

    # 书稿词表整表前置：方言读法先于两套语料和同音字表生效；同形多读音按书稿顺序，
    # 完全相同的 (词形,读音) 对不再重复记入 multi。
    books4_rows = 0
    books_path = pathlib.Path(a.books)
    if books_path.exists():
        with books_path.open(encoding="utf-8") as f:
            for line in f:
                line = line.rstrip("\n")
                if "\t" not in line:
                    continue
                form, note = line.split("\t", 1)
                add(form, note, "", {"编号": "books4", "注例": ""}, "books4", skip_dupe=True)
                books4_rows += 1

    danzi = bp.load(a.corpus)
    char_keys = set()
    for form, note, ipa, rec in char_entries(danzi):
        add(form, note, ipa, rec, "单字")
        char_keys.add(form)

    words = bp.load(a.words)
    for form, note, ipa, rec in word_entries(words, unresolved):
        add(form, note, ipa, rec, "词汇", rec.get("词汇", ""))

    book_keys = set()
    for form, note, ipa, rec in tongyin_entries(pathlib.Path(a.tongyin), unresolved):
        add(form, note, ipa, rec, "同音字表", skip_dupe=True)
        book_keys.add(form)

    _write_text(a.out, [f"{k}\t{v}\n" for k, v in multi])
    print(f"词表 {len(multi)} 行 → {a.out}（`bun run data` 会把它压成 .txt.gz）")

    with open(a.csv, "w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=CSV_FIELDS, lineterminator="\n")
        w.writeheader()
        w.writerows(csv_rows)
    with open(a.unresolved, "w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["编号", "词汇", "说法", "读音", "原因"], lineterminator="\n")
        w.writeheader()
        w.writerows(unresolved)

    counts = Counter(k for k, _ in multi)
    multi_form = sum(1 for n in counts.values() if n > 1)
    word_rows = len([r for r in csv_rows if r["来源"] == "词汇"])
    word_forms = len({r["词形"] for r in csv_rows if r["来源"] == "词汇"})
    book_rows = len([r for r in csv_rows if r["来源"] == "同音字表"])
    book_forms = len({r["词形"] for r in csv_rows if r["来源"] == "同音字表"})
    book_only = book_keys - char_keys
    print(f"单字 {len(char_keys)} 字，词形 {word_forms} 个 / {word_rows} 条")
    if book_forms:
        print(f"同音字表 补 {book_forms} 字 / {book_rows} 条（其中语料没有的新字 {len(book_only)} 个）")
    print(f"单字总计 {len({r['词形'] for r in csv_rows if len(r['词形']) == 1})} 字")
    print(f"词表 {len(multi)} 行（{multi_form} 个词形多读）→ {a.out}")
    if books4_rows:
        print(f"书稿 books4 前置 {books4_rows} 行（{len({r['词形'] for r in csv_rows if r['来源'] == 'books4'})} 个词形）")
    print(f"溯源 {len(csv_rows)} 行 → {a.csv}；未解析 {len(unresolved)} 条 → {a.unresolved}")
    if any(r["原因"].startswith("映射不上") or "4 列" in r["原因"] for r in unresolved):
        print("同音字表有映射失败的行，见 unresolved", file=sys.stderr)
        return 2
    return 0


def _write_text(path: str, lines: list[str]) -> None:
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("".join(lines))


if __name__ == "__main__":
    main()
