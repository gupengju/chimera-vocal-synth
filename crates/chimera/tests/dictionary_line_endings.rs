//! 词典行尾：CRLF 与 LF 必须读成同一张表。C++ 移植版按 `getline` + `>>` 解析，天然吞掉 `\r`；
//! Rust 侧曾用 `split('\n')`，CRLF 词典会让每行的最后一个音节带上 `\r` 而解析失败
//! （"你好" 只剩 "ni3"，"中华人民共和国" 少 "国"）。这个测试把两侧的约定钉住。
use chimera::phonetics::guiyang::GuiyangG2P;
use chimera::phonetics::mandarin::MandarinG2P;
use chimera::phonetics::english::Phrase;

fn shown(phrases: &[Phrase]) -> String {
    phrases
        .iter()
        .map(|p| {
            p.segments
                .iter()
                .map(|s| format!("{}{}", s.ipa, s.tone.map_or(String::new(), |t| t.to_string())))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

const LF: &str = "你\tni3\n好\thao3\n中华人民共和国\tzhong1 hua2 ren2 min2 gong4 he2 guo2\n国\tgue2\n角\tjio2\n";
const CRLF: &str =
    "你\tni3\r\n好\thao3\r\n中华人民共和国\tzhong1 hua2 ren2 min2 gong4 he2 guo2\r\n国\tgue2\r\n角\tjio2\r\n";

#[test]
fn crlf_and_lf_dictionaries_agree() {
    let mut a = MandarinG2P::new();
    a.load_dict(LF);
    let mut b = MandarinG2P::new();
    b.load_dict(CRLF);
    for text in ["你好。", "中华人民共和国", "你好，世界。"] {
        assert_eq!(shown(&a.to_phrases(text)), shown(&b.to_phrases(text)), "{text:?}");
    }

    let mut c = GuiyangG2P::new();
    c.load_dict(LF);
    let mut d = GuiyangG2P::new();
    d.load_dict(CRLF);
    for text in ["国角。", "你好。", "gue2 jio2"] {
        assert_eq!(shown(&c.to_phrases(text)), shown(&d.to_phrases(text)), "{text:?}");
    }
}

#[test]
fn crlf_keeps_every_syllable() {
    let mut g = MandarinG2P::new();
    g.load_dict(CRLF);
    // 六字词：CRLF 未处理时最后一个音节会丢
    let p = g.to_phrases("中华人民共和国");
    let n: usize = p.iter().map(|x| x.segments.len()).sum();
    assert!(n >= 10, "音节数 {n} 太少，说明行尾 \\r 仍在破坏读音");
}
