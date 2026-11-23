//! Guiyang (贵阳) new-school speech: the book's pinyin-like notation → phoneme segments with four
//! tones. The notation is documented in `docs/DIALECT_NOTATION.md` of the C++ port; both trees
//! implement the same tables ( `tools/notation_check.py` pins them to `ipa5.csv`).
//!
//! Differences from Mandarin pinyin (not quirks — these are the dialect's phonology):
//!   * `b d g` are voiceless unaspirated [p t k]; `z c s` are alveolar [ts tsʰ s], `r` is [z];
//!   * `e` is [ɛ], `o` is [o] (no [w] glide), `ong` is [oŋ], `iong` is [ioŋ];
//!   * `iu` is [iəu] (as in Mandarin) and `iw` is the dialect's [iu]; `io`, `ue` are new rhymes;
//!   * `ŋ` onsets are spelled `ng` (安 ngan¹ = [ŋan]); a bare final (a¹) is true zero-initial,
//!     so 爱 ngai⁴ [ŋai] and ai⁴ [ai] stay distinct.
//!
//! Tones are 55 / 21 / 42 / 13 (planner `ToneScheme::Guiyang`). No tone sandhi yet.

use super::english::Phrase;
use super::{Intonation, Segment};
use std::collections::HashMap;

/// A syllable's segments, tone and the character it was read from.
struct Syl {
    segs: Vec<Segment>,
    tone: u8,
}

fn is_han(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c) || c == '〇'
}

/// initial spelling → (IPA, aspirated). `None` = no aspiration contrast (sonorants, fricatives).
fn initial(i: &str) -> Option<(&'static str, Option<bool>)> {
    Some(match i {
        "b" => ("p", Some(false)),
        "p" => ("p", Some(true)),
        "m" => ("m", None),
        "f" => ("f", None),
        "d" => ("t", Some(false)),
        "t" => ("t", Some(true)),
        "n" => ("n", None),
        "l" => ("l", None),
        "g" => ("k", Some(false)),
        "k" => ("k", Some(true)),
        "h" => ("x", None),
        "j" => ("tɕ", Some(false)),
        "q" => ("tɕ", Some(true)),
        "x" => ("ɕ", None),
        // the book's ʦ/ʦʰ are alveolar, not the dental [ts̪] of Mandarin z/c
        "z" => ("ts", Some(false)),
        "c" => ("ts", Some(true)),
        "s" => ("s", None),
        "r" => ("z", None),
        // 书上的零声母写法不带字母，真正的 ŋ- 声母写 ng（安 ngan¹ = [ŋan]，爱 ngai⁴ 与 ai⁴ 对立）
        "ng" => ("ŋ", None),
        _ => return None,
    })
}

/// Canonical final → segments. A run of vowels stays one segment (the planner expands it with
/// equal weights); a nasal coda is its own segment.
fn final_ipa(f: &str) -> Option<&'static [&'static str]> {
    Some(match f {
        "a" => &["a"],
        "o" => &["o"],
        "e" => &["ɛ"],
        "i" => &["i"],
        "u" => &["u"],
        "ü" => &["y"],
        "ai" => &["ai"],
        "ei" => &["ei"],
        "ao" => &["ɑu"],
        "ou" => &["əu"],
        "an" => &["a", "n"],
        "en" => &["ə", "n"],
        "ang" => &["ɑ", "ŋ"],
        "ong" => &["o", "ŋ"],
        "er" => &["ɚ"],
        "ia" => &["ia"],
        "ie" => &["iɛ"],
        "io" => &["io"],
        "iw" => &["iu"],
        "iu" => &["iəu"],
        "iao" => &["iɑu"],
        "ian" => &["ia", "n"],
        "in" => &["i", "n"],
        "iang" => &["iɑ", "ŋ"],
        "iong" => &["io", "ŋ"],
        "ua" => &["ua"],
        "ue" => &["uɛ"],
        "ui" => &["uei"],
        "uai" => &["uai"],
        "uan" => &["ua", "n"],
        "un" => &["uə", "n"],
        "uang" => &["uɑ", "ŋ"],
        "üe" => &["yɛ"],
        "üan" => &["ya", "n"],
        "ün" => &["y", "n"],
        // the apical vowel of zi/ci/si/ri, written ɿ in the book
        "-i" => &["ɨ"],
        _ => return None,
    })
}

/// Zero-initial spellings: the y/w letter is orthography only, the value has no medial.
fn final_zero(s: &str) -> Option<&'static [&'static str]> {
    Some(match s {
        "a" | "ai" | "an" | "ang" | "ao" | "e" | "en" | "er" | "o" | "ou" => final_ipa(s)?,
        "ong" => &["o", "ŋ"],
        "yi" => &["i"],
        "ya" => &["ia"],
        "ye" => &["iɛ"],
        "yo" => &["io"],
        "yao" => &["iɑu"],
        "you" => &["iəu"],
        "yan" => &["ia", "n"],
        "yin" => &["i", "n"],
        "yang" => &["iɑ", "ŋ"],
        "yong" => &["io", "ŋ"],
        "yu" => &["y"],
        "yue" => &["yɛ"],
        "yuan" => &["ya", "n"],
        "yun" => &["y", "n"],
        "wu" => &["u"],
        "wa" => &["ua"],
        "we" => &["uɛ"],
        "wai" => &["uai"],
        "wei" => &["uei"],
        "wan" => &["ua", "n"],
        "wen" => &["uə", "n"],
        "wang" => &["uɑ", "ŋ"],
        _ => return None,
    })
}

/// Tone-marked vowel → (vowel, tone).
fn unmark(c: char) -> Option<(char, u8)> {
    const MARKS: [(&str, char); 6] = [("āáǎà", 'a'), ("ēéěè", 'e'), ("īíǐì", 'i'), ("ōóǒò", 'o'), ("ūúǔù", 'u'), ("ǖǘǚǜ", 'ü')];
    MARKS.iter().find_map(|(m, v)| m.chars().position(|x| x == c).map(|k| (*v, k as u8 + 1)))
}

/// One syllable (lowercase, tone mark or trailing digit) → its segments and tone.
pub fn syllable(s: &str) -> Option<(Vec<Segment>, u8)> {
    let mut tone = 5u8;
    let mut base = String::new();
    for c in s.chars() {
        if let Some((v, t)) = unmark(c) {
            base.push(v);
            tone = t;
        } else if let Some(d) = c.to_digit(10) {
            tone = if (1..=4).contains(&d) { d as u8 } else { 5 };
        } else {
            base.push(if c == 'v' { 'ü' } else { c });
        }
    }
    // 声母：两字母的 ng 优先，其余单字母（zh/ch/sh 已并入别的声母）
    let ini = if base.starts_with("ng") {
        Some("ng")
    } else {
        base.get(..1).filter(|i| "bpmfdtnlgkhjqxzcsr".contains(*i))
    };
    let (ini, rest) = match ini {
        Some(i) => (i, &base[i.len()..]),
        None => ("", base.as_str()),
    };
    let parts: &[&str] = if ini.is_empty() {
        // y/w spellings first; a bare final ("ian", "u") is the same value without medial letters
        final_zero(base.as_str()).or_else(|| final_ipa(base.as_str()))?
    } else {
        // j/q/x + u is [y] (ju qu xu = [tɕy tɕʰy ɕy]); z/c/s/r + i is the apical vowel
        let fin: String = if matches!(ini, "j" | "q" | "x") && rest.starts_with('u') {
            format!("ü{}", &rest[1..])
        } else if matches!(ini, "z" | "c" | "s" | "r") && rest == "i" {
            "-i".into()
        } else {
            rest.to_string()
        };
        final_ipa(&fin)?
    };
    let mut out = Vec::with_capacity(4);
    if !ini.is_empty() {
        let (ipa, aspirated) = initial(ini)?;
        out.push(Segment { aspirated, ..Segment::new(ipa) });
    }
    for p in parts {
        let nucleus = !matches!(*p, "n" | "ŋ");
        out.push(if nucleus { Segment { tone: Some(tone), stress: Some(if tone < 5 { 1 } else { 0 }), ..Segment::new(p) } } else { Segment::new(p) });
    }
    Some((out, tone))
}

/// A word → syllables: split at apostrophes and after tone digits, then by longest match.
fn split_word(w: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut chunk = String::new();
    let mut chunks = Vec::new();
    for c in w.chars() {
        if c == '\'' || c == '’' || c == '-' {
            chunks.push(std::mem::take(&mut chunk));
            continue;
        }
        chunk.push(c);
        if c.is_ascii_digit() {
            chunks.push(std::mem::take(&mut chunk));
        }
    }
    chunks.push(chunk);
    for c in chunks.iter().filter(|c| !c.is_empty()) {
        let letters = c.trim_end_matches(|x: char| x.is_ascii_digit());
        let cs: Vec<char> = letters.chars().collect();
        let mut i = 0;
        while i < cs.len() {
            let n = (1..=(cs.len() - i).min(6)).rev().find(|&n| syllable(&cs[i..i + n].iter().collect::<String>()).is_some())?;
            out.push(cs[i..i + n].iter().collect::<String>());
            i += n;
        }
        if let Some(last) = out.last_mut() {
            last.push_str(&c[letters.len()..]);
        }
    }
    Some(out)
}

fn is_phrase_punct(c: char) -> bool {
    ".!?;:,，。！？；：、".contains(c)
}

fn is_pinyin_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '\'' || c == '’' || c == 'ü' || unmark(c).is_some()
}

/// Whether text is Guiyang pinyin (or characters): tone digits, tone marks or Han characters.
pub fn is_guiyang(text: &str) -> bool {
    text.chars().any(|c| is_han(c) || unmark(c).is_some()) || text.split(' ').any(|w| split_word(w).is_some())
}

/// Segment durations: the Mandarin speech model (fixed nucleus length twice a real speaker's,
/// real mean consonant lengths) with the dialect's phoneme names. See `mandarin.rs` for the
/// measurements that motivate the numbers.
fn speech_durations(s: &mut Syl, last: bool) {
    let coda = s.segs.last().is_some_and(|x| x.ipa == "n" || x.ipa == "ŋ");
    let nucleus = s.segs.iter().position(|x| x.tone.is_some()).unwrap_or(s.segs.len());
    for (i, x) in s.segs.iter_mut().enumerate() {
        let ms = match x.ipa.as_str() {
            _ if i == nucleus => {
                let mut d = if s.tone == 5 { 120.0 } else { 150.0 };
                if x.ipa.chars().count() > 1 {
                    d += 40.0;
                }
                if coda {
                    d -= 30.0;
                }
                if last { d * 1.5 } else { d }
            }
            "p" | "t" | "k" => {
                if x.aspirated == Some(true) { 90.0 } else { 58.0 }
            }
            "ts" | "tɕ" if x.aspirated == Some(true) => 101.0,
            "ts" | "tɕ" => 73.0,
            "f" | "s" | "ɕ" | "x" => 100.0,
            "m" => 63.0,
            "l" => 46.0,
            "z" => 67.0,
            "n" | "ŋ" if i > nucleus => 67.0 * if last { 1.56 } else { 1.0 } * if s.tone == 5 { 0.85 } else { 1.0 },
            "n" => 50.0,
            "j" | "w" | "ɥ" if i > 0 => 30.0,
            _ => 57.0,
        };
        x.dur = Some(ms / 1000.0);
    }
}

/// Guiyang text (characters or notation) → phrases. Characters need the dictionary
/// (`public/data/guiyang.txt.gz`); the notation works without it.
#[derive(Clone, Debug, Default)]
pub struct GuiyangG2P {
    dict: Option<HashMap<String, String>>,
    max_len: usize,
}

impl GuiyangG2P {
    pub fn new() -> GuiyangG2P {
        GuiyangG2P::default()
    }

    /// The dictionary as text ("词形\t读音" lines). 同一词形可以有多行（多读音），先出现的
    /// 那条是默认读音，后面的忽略——装载顺序按语料顺序，不按行尾覆盖。
    pub fn load_dict(&mut self, text: &str) {
        let mut m = HashMap::new();
        let mut max_len = 1;
        for line in text.split('\n') {
            if let Some((k, v)) = line.split_once('\t') {
                max_len = max_len.max(k.chars().count());
                m.entry(k.to_string()).or_insert_with(|| v.to_string());
            }
        }
        self.dict = Some(m);
        self.max_len = max_len;
    }

    /// Text → phrases (punctuation breaks phrases; "?" makes a question). A digit not after a
    /// pinyin letter is read as its character (2026 → 二零二六).
    pub fn to_phrases(&self, text: &str) -> Vec<Phrase> {
        let mut phrases = Vec::new();
        let mut cur = String::new();
        let mut prev: Option<char> = None;
        for c in text.chars() {
            if is_phrase_punct(c) {
                self.flush(&mut cur, Some(c), &mut phrases);
            } else if let Some(d) = c.to_digit(10).filter(|_| !prev.is_some_and(char::is_alphabetic)) {
                cur.push("零一二三四五六七八九".chars().nth(d as usize).unwrap());
            } else {
                cur.push(c);
            }
            prev = Some(c);
        }
        self.flush(&mut cur, None, &mut phrases);
        phrases
    }

    /// Characters → dictionary words by forward maximum matching (unknown characters are skipped).
    fn han_words(&self, run: &[char], words: &mut Vec<(String, Vec<Syl>)>) {
        let Some(dict) = &self.dict else { return };
        let mut i = 0;
        while i < run.len() {
            let found = (1..=self.max_len.min(run.len() - i)).rev().find_map(|n| {
                let w: String = run[i..i + n].iter().collect();
                dict.get(&w).map(|p| (w, n, p))
            });
            let Some((w, n, p)) = found else {
                i += 1;
                continue;
            };
            let readings: Vec<&str> = p.split_whitespace().collect();
            let syls: Vec<Syl> = readings.iter().filter_map(|r| syllable(r).map(|(segs, tone)| Syl { segs, tone })).collect();
            // 词形与音节数不必相等：合音词（不要 → biao4）两字一音节
            if !syls.is_empty() && syls.len() == readings.len() {
                words.push((w, syls));
            }
            i += n;
        }
    }

    fn flush(&self, cur: &mut String, delim: Option<char>, phrases: &mut Vec<Phrase>) {
        let mut words = self.words(cur);
        cur.clear();
        // no tone sandhi in this dialect (21+21 → 21+55 is a later stage)
        let n: usize = words.iter().map(|w| w.1.len()).sum();
        let mut k = 0;
        for w in words.iter_mut() {
            for s in w.1.iter_mut() {
                k += 1;
                speech_durations(s, k == n);
            }
        }
        let mut segs = Vec::new();
        let mut said = Vec::new();
        for (w, ss) in &words {
            let start = segs.len();
            for s in ss {
                segs.extend(s.segs.iter().cloned());
            }
            if segs.len() > start {
                segs.last_mut().unwrap().word_end = Some(true);
                said.push(w.clone());
            }
        }
        if segs.is_empty() {
            return;
        }
        let q = matches!(delim, Some('?' | '？'));
        let pause = match delim {
            Some('.' | '!' | '?' | '。' | '！' | '？') | None => 0.35,
            _ => 0.18,
        };
        phrases.push(Phrase { segments: segs, intonation: if q { Intonation::Question } else { Intonation::Statement }, pause, words: said });
    }

    /// Text without phrase punctuation → words of syllables (dictionary words, notation words).
    fn words(&self, text: &str) -> Vec<(String, Vec<Syl>)> {
        let mut words: Vec<(String, Vec<Syl>)> = Vec::new();
        let chars: Vec<char> = text.to_lowercase().chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let start = i;
            if is_han(chars[i]) {
                while i < chars.len() && is_han(chars[i]) {
                    i += 1;
                }
                self.han_words(&chars[start..i], &mut words);
            } else if is_pinyin_char(chars[i]) {
                while i < chars.len() && is_pinyin_char(chars[i]) {
                    i += 1;
                }
                let w: String = chars[start..i].iter().collect();
                if let Some(syls) = split_word(&w) {
                    words.push((w, syls.iter().filter_map(|s| syllable(s).map(|(segs, tone)| Syl { segs, tone })).collect()));
                }
            } else {
                i += 1;
            }
        }
        words
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ipa(s: &str) -> String {
        let (segs, t) = syllable(s).unwrap();
        let mut o: String = segs.iter().map(|x| format!("{}{}", x.ipa, if x.aspirated == Some(true) { "ʰ" } else { "" })).collect();
        o.push_str(&t.to_string());
        o
    }

    #[test]
    fn syllables() {
        assert_eq!(ipa("jio2"), "tɕio2");
        assert_eq!(ipa("qiw2"), "tɕʰiu2");
        assert_eq!(ipa("gue2"), "kuɛ2");
        assert_eq!(ipa("be2"), "pɛ2");
        assert_eq!(ipa("o2"), "o2");
        assert_eq!(ipa("zong1"), "tsoŋ1");
        assert_eq!(ipa("zi1"), "tsɨ1");
        assert_eq!(ipa("liu2"), "liəu2");
        assert_eq!(ipa("ju4"), "tɕy4");
        assert_eq!(ipa("jue2"), "tɕyɛ2");
        assert_eq!(ipa("juan2"), "tɕyan2");
        assert_eq!(ipa("yuan2"), "yan2");
        assert_eq!(ipa("we2"), "uɛ2");
        assert_eq!(ipa("er4"), "ɚ4");
        // ŋ 声母写 ng，裸韵母是零声母：两者不合并
        assert_eq!(ipa("nga1"), "ŋa1");
        assert_eq!(ipa("ngan1"), "ŋan1");
        assert_eq!(ipa("ngo2"), "ŋo2");
        assert_eq!(ipa("ngiu3"), "ŋiəu3");
        assert_eq!(ipa("a1"), "a1");
        assert!(syllable("zhong1").is_none()); // 方言没有 zh
    }

    #[test]
    fn every_syllable_has_one_toned_nucleus() {
        for s in ["a", "jio", "qiw", "gue", "iang", "iong", "uan", "yu", "zi"] {
            let (segs, _) = syllable(s).unwrap();
            assert_eq!(segs.iter().filter(|x| x.tone.is_some()).count(), 1, "{s}");
        }
    }

    #[test]
    fn words() {
        assert_eq!(split_word("jio2").unwrap(), vec!["jio2".to_string()]);
        assert_eq!(split_word("zhong1").is_none(), true); // 方言没有 zh
        let p = GuiyangG2P::new().to_phrases("gue2 jio2, qiw2 ma?");
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].segments.len(), 4); // k uɛ | tɕ io
        assert_eq!(p[1].intonation, Intonation::Question);
    }
}
