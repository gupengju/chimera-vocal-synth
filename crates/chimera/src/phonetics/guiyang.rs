//! Guiyang (贵阳) new-school speech: the book's pinyin-like notation → phoneme segments with four
//! tones. The notation is documented in `docs/DIALECT_NOTATION.md` of the C++ port; both trees
//! implement the same tables ( `tools/notation_check.py` pins them to `ipa5.csv`).
//!
//! **The tables are not here.** Initials, finals, the y/w spellings, the tone letters and the
//! durations live in `public/data/guiyang.rules.txt`, parsed by [`super::guiyang_rules`]. This
//! module only does the text side: characters → dictionary words, punctuation → phrases, and the
//! per-syllable timing that the dialect's fixed nucleus length needs.
//!
//! Differences from Mandarin pinyin (not quirks — these are the dialect's phonology):
//!   * `b d g` are voiceless unaspirated [p t k]; `z c s` are alveolar [ts tsʰ s], `r` is [z];
//!   * `e` is [ɛ], `o` is [o] (no [w] glide), `ong` is [oŋ], `iong` is [ioŋ];
//!   * `iu` is [iəu] (as in Mandarin) and `iw` is the dialect's [iu]; `io`, `ue` are new rhymes;
//!   * `ŋ` onsets are spelled `ng` (安 ngan¹ = [ŋan]); a bare final (a¹) is true zero-initial,
//!     so 爱 ngai⁴ [ŋai] and ai⁴ [ai] stay distinct.
//!   * `ŋ` is a consonant segment on the same tier as m/n/p/t; the following vowel stays oral.
//!     The onset ŋ does NOT nasalize the vowel — vowel nasalization (ã õ …) is a separate
//!     rhyme-layer feature, never attached to an ŋ onset (ngo³ = ŋ + o, not ŋ + õ).
//!
//! Tones are 55 / 21 / 42 / 13 (planner `ToneScheme::Guiyang`). No tone sandhi yet.

use super::english::Phrase;
use super::guiyang_rules::{is_han, GuiyangRules};
use super::{Intonation, Segment};
use std::collections::HashMap;

/// A syllable's segments, tone and the character it was read from.
struct Syl {
    segs: Vec<Segment>,
    tone: u8,
}

fn is_phrase_punct(c: char) -> bool {
    ".!?;:,，。！？；：、".contains(c)
}

/// Guiyang text (characters or notation) → phrases. Characters need the dictionary
/// (`public/data/guiyang.txt.gz`); the notation works without it.
#[derive(Clone, Debug, Default)]
pub struct GuiyangG2P {
    dict: Option<HashMap<String, String>>,
    max_len: usize,
    rules: GuiyangRules,
}

impl GuiyangG2P {
    pub fn new() -> GuiyangG2P {
        GuiyangG2P::default()
    }

    /// The tables in force (the embedded copy until [`Self::set_rules_text`] replaces them).
    pub fn rules(&self) -> &GuiyangRules {
        &self.rules
    }

    /// Replace the tables from the text of `public/data/guiyang.rules.txt`. A bad file is an error
    /// (with the line number) and leaves the previous tables in force.
    pub fn set_rules_text(&mut self, text: &str) -> Result<(), String> {
        self.rules = GuiyangRules::parse(text)?;
        Ok(())
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
                cur.push(self.rules.digits[d as usize]);
            } else {
                cur.push(c);
            }
            prev = Some(c);
        }
        self.flush(&mut cur, None, &mut phrases);
        phrases
    }

    /// Whether text is Guiyang notation or characters.
    pub fn is_guiyang(&self, text: &str) -> bool {
        self.rules.is_guiyang(text)
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
            let syls: Vec<Syl> = readings.iter().filter_map(|r| self.rules.syllable(r).map(|(segs, tone)| Syl { segs, tone })).collect();
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
                self.rules.speech_durations(&mut s.segs, s.tone, k == n);
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
        let pause = if matches!(delim, Some('.' | '!' | '?' | '。' | '！' | '？') | None) { self.rules.pause_sentence } else { self.rules.pause_clause };
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
            } else if self.rules.is_pinyin_char(chars[i]) {
                while i < chars.len() && self.rules.is_pinyin_char(chars[i]) {
                    i += 1;
                }
                let w: String = chars[start..i].iter().collect();
                if let Some(syls) = self.rules.split_word(&w) {
                    words.push((w, syls.iter().filter_map(|s| self.rules.syllable(s).map(|(segs, tone)| Syl { segs, tone })).collect()));
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
    use crate::phonetics::guiyang_rules::GuiyangRules;

    /// The rules as they are on disk now (not the copy embedded at build time).
    fn rules() -> GuiyangRules {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../public/data/guiyang.rules.txt");
        GuiyangRules::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn ipa(s: &str) -> String {
        let (segs, t) = rules().syllable(s).unwrap();
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
        assert_eq!(ipa("ngo3"), "ŋo3");
        assert_eq!(ipa("ngiu3"), "ŋiəu3");
        assert_eq!(ipa("a1"), "a1");
        assert!(rules().syllable("zhong1").is_none()); // 方言没有 zh
    }

    #[test]
    fn ŋ_onset_is_consonant_vowel_stays_oral() {
        // ngo³ = ŋ + o：ŋ 是独立辅音音段（与 m/n/p/t 同级），后面的 o 是口元音，
        // 不带鼻化；ŋ 声母不触发元音鼻化（鼻化是韵母层的独立特征）。
        let (segs, tone) = rules().syllable("ngo3").unwrap();
        assert_eq!(tone, 3);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].ipa, "ŋ");
        assert!(segs[0].aspirated.is_none()); // 响音不送气
        assert!(segs[0].tone.is_none()); // 辅音不承担声调
        assert_eq!(segs[1].ipa, "o");
        assert!(segs[1].tone.is_some()); // o 是带调韵核
        assert!(!["n", "ŋ"].contains(&segs[1].ipa.as_str()));
        assert!(!segs[1].ipa.contains('\u{0303}')); // 无鼻化波浪号

        // ngan¹ = ŋ + a + n：鼻音色彩来自末尾 n 韵尾，不是 ŋ 声母把 a 鼻化。
        let (segs, _) = rules().syllable("ngan1").unwrap();
        assert_eq!(segs.iter().map(|s| s.ipa.as_str()).collect::<Vec<_>>(), vec!["ŋ", "a", "n"]);
        assert_eq!(segs[1].ipa, "a");
        assert!(!segs[1].ipa.contains('\u{0303}'));
    }

    #[test]
    fn every_syllable_has_one_toned_nucleus() {
        for s in ["a", "jio", "qiw", "gue", "iang", "iong", "uan", "yu", "zi"] {
            let (segs, _) = rules().syllable(s).unwrap();
            assert_eq!(segs.iter().filter(|x| x.tone.is_some()).count(), 1, "{s}");
        }
    }

    #[test]
    fn words() {
        let r = rules();
        assert_eq!(r.split_word("jio2").unwrap(), vec!["jio2".to_string()]);
        assert!(r.split_word("zhong1").is_none()); // 方言没有 zh
        let p = GuiyangG2P::new().to_phrases("gue2 jio2, qiw2 ma?");
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].segments.len(), 4); // k uɛ | tɕ io
        assert_eq!(p[1].intonation, Intonation::Question);
    }

    #[test]
    fn the_rules_file_is_the_source_of_truth() {
        // 文件里的表就是引擎的表：新增一个韵母不用改 Rust。
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../public/data/guiyang.rules.txt");
        let mut text = std::fs::read_to_string(path).unwrap();
        text = text.replace("[finals]\n", "[finals]\nzz  k\n");
        let edited = GuiyangRules::parse(&text).unwrap();
        let (segs, _) = edited.syllable("bzz1").unwrap();
        assert_eq!(segs.iter().map(|s| s.ipa.as_str()).collect::<Vec<_>>(), vec!["p", "k"]);
        // 运行时的替换走的是同一个解析器
        let mut g = GuiyangG2P::new();
        g.set_rules_text(&text).unwrap();
        assert_eq!(g.rules().syllable("bzz1").unwrap().0.len(), 2);
    }

    #[test]
    fn a_broken_line_names_its_line() {
        let err = GuiyangRules::parse("[syllable]\nmax_length 6\n[initials]\nz\n").unwrap_err();
        assert!(err.contains(":4:"), "{err}");
    }
}
