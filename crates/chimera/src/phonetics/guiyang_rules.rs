//! The Guiyang notation's rules, parsed from `public/data/guiyang.rules.txt`.
//!
//! The tables used to live in `guiyang.rs` as `match` arms; they are now a text file so a
//! phonologist can edit the dialect's phonology without touching Rust. The same text is
//! embedded here (`include_str!`) as a fallback for a build that ships without `public/`,
//! and `guiyangRules` (wasm) / [`GuiyangG2P::set_rules_text`] replace it at run time.
//!
//! What the file owns: initials (and their aspiration), finals (and how a final expands into
//! phonemes), the y/w spellings of zero-initial syllables, the j/q/x + u and z/c/s/r + i
//! rewrites, the tone-mark letters, the tone contours ([`Melody`]) with their 变音 rules, the
//! tone-digit alphabet, the digit readings, the phrase pauses, and the per-phoneme durations.
//! Glyph classification (what is a Han character, what is punctuation) stays in code — that is
//! not phonology.

use super::Segment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The shipped rules, compiled in as a fallback (the file itself is the run-time source).
const EMBEDDED: &str = include_str!("../../../../public/data/guiyang.rules.txt");

/// One notation letter (or digraph) and the phoneme it spells.
#[derive(Clone, Debug, PartialEq)]
pub struct Initial {
    pub key: String,
    pub ipa: String,
    /// `None`: this class has no aspiration contrast (sonorants, fricatives)
    pub aspirated: Option<bool>,
}

/// Segment durations, milliseconds (see `[timing]`).
#[derive(Clone, Debug)]
pub struct Timing {
    pub default: f64,
    pub vowel: f64,
    pub vowel_neutral: f64,
    pub diphthong_extra: f64,
    pub coda_shorten: f64,
    pub last_factor: f64,
    pub coda_nasal: f64,
    pub coda_last_factor: f64,
    pub coda_neutral_factor: f64,
    pub glide: f64,
    pub glide_phones: Vec<String>,
    /// phoneme → (plain, aspirated) ms
    pub phones: HashMap<String, (f64, Option<f64>)>,
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            default: 57.0,
            vowel: 150.0,
            vowel_neutral: 120.0,
            diphthong_extra: 40.0,
            coda_shorten: 30.0,
            last_factor: 1.5,
            coda_nasal: 67.0,
            coda_last_factor: 1.56,
            coda_neutral_factor: 0.85,
            glide: 30.0,
            glide_phones: vec!["j".into(), "w".into(), "ɥ".into()],
            phones: HashMap::new(),
        }
    }
}

/// A tone's pitch targets and the dialect's 变音 (tone-changing) rules, from `[tones]` and
/// `[melody]`. Levels are Chao's five (1–5); the planner turns each into
/// `base · 2^((level − 3) · chaostep / 12)` semitones.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Melody {
    /// tone number → its levels, spread evenly over the rhyme ([tones])
    pub contours: Vec<(u8, Vec<f64>)>,
    /// the neutral tone's level after tone 1, 2, 3, 4, and with nothing before ([melody])
    pub neutral: Vec<f64>,
    /// semitones per Chao level: how wide the speaker's tonal range is
    pub chaostep: f64,
    /// how far the phrase's pitch falls from its first syllable to its last, semitones
    pub declination: f64,
    /// extra semitones on a question's last syllable
    pub question_rise: f64,
}

impl Default for Melody {
    /// The Guiyang defaults, identical to what the planner hard-coded before the file owned them.
    fn default() -> Self {
        Melody {
            contours: vec![(1, vec![5.0, 5.0]), (2, vec![2.0, 1.0]), (3, vec![4.0, 2.0]), (4, vec![1.0, 3.0])],
            neutral: vec![2.0, 3.0, 4.0, 1.0, 3.0],
            chaostep: 2.5,
            declination: 1.7,
            question_rise: 3.0,
        }
    }
}

impl Melody {
    /// The pitch targets of `tone` (a tone the table does not list is the neutral tone).
    pub fn contour(&self, tone: u8, prev: Option<u8>) -> Vec<f64> {
        match self.contours.iter().find(|(t, _)| *t == tone) {
            Some((_, levels)) => levels.clone(),
            None => self.neutral_after(prev),
        }
    }

    /// The neutral tone has no level of its own: it takes the tail of the tone before it.
    pub fn neutral_after(&self, prev: Option<u8>) -> Vec<f64> {
        let i = match prev {
            Some(1) => 0,
            Some(2) => 1,
            Some(3) => 2,
            Some(4) => 3,
            _ => 4,
        };
        vec![self.neutral.get(i).copied().unwrap_or(3.0)]
    }
}

/// Everything `guiyang.rules.txt` declares.
#[derive(Clone, Debug)]
pub struct GuiyangRules {
    /// longest syllable spelling, in letters
    pub max_syllable: usize,
    /// final-internal consonants (they take no tone and are not nuclei)
    pub coda_phones: Vec<String>,
    /// the letter that stands in for ü on a plain keyboard
    pub v_as: char,    /// longest key first, so `ng` wins over `n`
    pub initials: Vec<Initial>,
    pub finals: HashMap<String, Vec<String>>,
    pub finals_zero: HashMap<String, Vec<String>>,
    /// initials whose `u` is really a ü rhyme
    pub palatal_u: Vec<String>,
    /// initials whose bare `i` is the apical vowel, and the final key to use
    pub apical_i: Vec<(String, String)>,
    /// tone-marked letter → (plain letter, tone 1–4)
    pub marks: Vec<(char, char, u8)>,
    pub neutral: u8,
    pub toned: Vec<u8>,
    /// the tone contours and the 变音 rules ([tones] + [melody])
    pub melody: Melody,
    /// 连读变调 rules: (tone before, tone, surface tone) ([sandhi])
    pub sandhi: Vec<(u8, u8, u8)>,
    /// 0–9 → the character that reads it
    pub digits: [char; 10],
    pub pause_sentence: f64,
    pub pause_clause: f64,
    pub timing: Timing,
}

impl Default for GuiyangRules {
    fn default() -> Self {
        GuiyangRules::builtin()
    }
}

impl GuiyangRules {
    /// The text embedded at build time, parsed.
    pub fn builtin() -> GuiyangRules {
        GuiyangRules::parse(EMBEDDED).unwrap_or_else(|e| panic!("embedded {e} (run the rules test)"))
    }

    pub fn parse(text: &str) -> Result<GuiyangRules, String> {
        let mut r = GuiyangRules {
            max_syllable: 6,
            coda_phones: Vec::new(),
            v_as: 'v',
            initials: Vec::new(),
            finals: HashMap::new(),
            finals_zero: HashMap::new(),
            palatal_u: Vec::new(),
            apical_i: Vec::new(),
            marks: Vec::new(),
            neutral: 5,
            toned: Vec::new(),
            melody: Melody { contours: Vec::new(), ..Melody::default() },
            sandhi: Vec::new(),
            digits: [' '; 10],
            pause_sentence: 0.35,
            pause_clause: 0.18,
            timing: Timing::default(),
        };
        let mut section = String::new();
        for (n, raw) in text.lines().enumerate() {
            let at = |m: String| format!("guiyang.rules.txt:{}: {m}", n + 1);
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') {
                if !line.ends_with(']') {
                    return Err(at(format!("unclosed section {line:?}")));
                }
                section = line[1..line.len() - 1].trim().to_string();
                continue;
            }
            let f: Vec<&str> = line.split_whitespace().collect();
            let num = |i: usize| -> Result<f64, String> {
                f.get(i).ok_or_else(|| at("missing a number".into()))?.parse::<f64>().map_err(|_| at(format!("{:?} is not a number", f[i])))
            };
            let word = |i: usize| -> Result<String, String> { f.get(i).map(|s| s.to_string()).ok_or_else(|| at("missing a value".into())) };
            let one = |i: usize| -> Result<char, String> {
                let s = f.get(i).ok_or_else(|| at("missing a character".into()))?;
                let mut cs = s.chars();
                match (cs.next(), cs.next()) {
                    (Some(c), None) => Ok(c),
                    _ => Err(at(format!("{s:?} is not one character"))),
                }
            };
            match section.as_str() {
                "syllable" => match f[0] {
                    "max_length" => r.max_syllable = num(1)? as usize,
                    "coda_phones" => r.coda_phones = f[1..].iter().map(|s| s.to_string()).collect(),
                    "v_as" => r.v_as = one(1)?,
                    k => return Err(at(format!("unknown [syllable] key {k:?}"))),
                },
                "initials" => {
                    let ipa = word(1)?;
                    let aspirated = match f.get(2) {
                        None => None,
                        Some(&"true") => Some(true),
                        Some(&"false") => Some(false),
                        Some(x) => return Err(at(format!("aspiration is true or false, not {x:?}"))),
                    };
                    r.initials.push(Initial { key: f[0].to_string(), ipa, aspirated });
                }
                "finals" | "finals.zero" => {
                    if f.len() < 2 {
                        return Err(at("a final needs at least one phoneme".into()));
                    }
                    let parts: Vec<String> = f[1..].iter().map(|s| s.to_string()).collect();
                    let map = if section == "finals" { &mut r.finals } else { &mut r.finals_zero };
                    map.insert(f[0].to_string(), parts);
                }
                "rewrite" => match f[0] {
                    "palatal_u" => r.palatal_u = f[1..].iter().map(|s| s.to_string()).collect(),
                    "apical_i" => {
                        let key = f.last().ok_or_else(|| at("apical_i needs a final key".into()))?;
                        let to = key.to_string();
                        for ini in &f[1..f.len() - 1] {
                            r.apical_i.push((ini.to_string(), to.clone()));
                        }
                    }
                    k => return Err(at(format!("unknown [rewrite] key {k:?}"))),
                },
                "marks" => {
                    let base = one(0)?;
                    let marked: Vec<char> = word(1)?.chars().collect();
                    if marked.len() != 4 {
                        return Err(at(format!("a vowel needs four tone marks, got {}", marked.len())));
                    }
                    for (i, m) in marked.into_iter().enumerate() {
                        r.marks.push((m, base, i as u8 + 1));
                    }
                }
                "tones" => {
                    if f[0] == "neutral" {
                        r.neutral = one(1)?.to_digit(10).ok_or_else(|| at("the neutral tone is a digit".into()))? as u8;
                    } else {
                        let t = f[0].parse::<u8>().map_err(|_| at(format!("{:?} is not a tone", f[0])))?;
                        let levels = word(1)?;
                        if levels.len() < 1 || levels.len() > 5 || !levels.chars().all(|c| ('1'..='5').contains(&c)) {
                            return Err(at(format!("tone {t}: levels are one to five digits 1–5, got {levels:?}")));
                        }
                        if r.melody.contours.iter().any(|(k, _)| *k == t) {
                            return Err(at(format!("tone {t} is declared twice")));
                        }
                        r.melody.contours.push((t, levels.chars().filter_map(|c| c.to_digit(10)).map(|d| d as f64).collect()));
                        r.toned.push(t);
                    }
                }
                "melody" => match f[0] {
                    // 变音规则：轻声跟着前字调的尾巴走，句首没有前字
                    k @ ("neutral_after1" | "neutral_after2" | "neutral_after3" | "neutral_after4" | "neutral_none") => {
                        let i = if k == "neutral_none" { 4 } else { (k.as_bytes()[k.len() - 1] - b'1') as usize };
                        if r.melody.neutral.len() < 5 {
                            r.melody.neutral.resize(5, 3.0);
                        }
                        r.melody.neutral[i] = num(1)?;
                    }
                    "declination" => r.melody.declination = num(1)?,
                    "question_rise" => r.melody.question_rise = num(1)?,
                    "chaostep" => r.melody.chaostep = num(1)?,
                    k => return Err(at(format!("unknown [melody] key {k:?}"))),
                },
                "sandhi" => {
                    // "前字+本字 变调后的调类"
                    let (p, t) = f[0]
                        .split_once('+')
                        .ok_or_else(|| at(format!("{:?} is not prev+tone", f[0])))?;
                    let prev = p.parse::<u8>().map_err(|_| at(format!("{:?} is not a tone", p)))?;
                    let tone = t.parse::<u8>().map_err(|_| at(format!("{:?} is not a tone", t)))?;
                    let surface = f[1].parse::<u8>().map_err(|_| at(format!("{:?} is not a tone", f[1])))?;
                    r.sandhi.push((prev, tone, surface));
                }
                "digits" => {
                    // "0 零": the digit, then the character that reads it
                    let d = f[0].chars().next().and_then(|c| c.to_digit(10)).ok_or_else(|| at(format!("{:?} is not a digit", f[0])))? as usize;
                    r.digits[d] = one(1)?;
                }
                "phrases" => match f[0] {
                    "pause_sentence" => r.pause_sentence = num(1)?,
                    "pause_clause" => r.pause_clause = num(1)?,
                    k => return Err(at(format!("unknown [phrases] key {k:?}"))),
                },
                "timing" => match f[0] {
                    "default" => r.timing.default = num(1)?,
                    "vowel" => r.timing.vowel = num(1)?,
                    "vowel_neutral" => r.timing.vowel_neutral = num(1)?,
                    "diphthong_extra" => r.timing.diphthong_extra = num(1)?,
                    "coda_shorten" => r.timing.coda_shorten = num(1)?,
                    "last_factor" => r.timing.last_factor = num(1)?,
                    "coda_nasal" => r.timing.coda_nasal = num(1)?,
                    "coda_last_factor" => r.timing.coda_last_factor = num(1)?,
                    "coda_neutral_factor" => r.timing.coda_neutral_factor = num(1)?,
                    "glide" => r.timing.glide = num(1)?,
                    "glide_phones" => r.timing.glide_phones = f[1..].iter().map(|s| s.to_string()).collect(),
                    k => return Err(at(format!("unknown [timing] key {k:?}"))),
                },
                "timing.phones" => {
                    let plain = num(1)?;
                    let asp = if f.len() > 2 { Some(num(2)?) } else { None };
                    r.timing.phones.insert(f[0].to_string(), (plain, asp));
                }
                "" => return Err(at(format!("{line:?} is outside a section"))),
                s => return Err(at(format!("unknown section [{s}]"))),
            }
        }
        if r.initials.is_empty() {
            return Err("guiyang.rules.txt: no [initials]".into());
        }
        if r.finals.is_empty() {
            return Err("guiyang.rules.txt: no [finals]".into());
        }
        if r.toned.is_empty() {
            return Err("guiyang.rules.txt: no tones in [tones]".into());
        }
        if r.digits.contains(&' ') {
            return Err("guiyang.rules.txt: [digits] is missing one of 0–9".to_string());
        }
        // the longest initial first, so `ng` is tried before `n`
        r.initials.sort_by_key(|i| std::cmp::Reverse(i.key.chars().count()));
        Ok(r)
    }

    pub fn is_coda(&self, ipa: &str) -> bool {
        self.coda_phones.iter().any(|c| c == ipa)
    }

    /// The tone a syllable takes after a syllable of tone `prev` ([sandhi]; itself when no rule).
    pub fn sandhi_after(&self, prev: u8, tone: u8) -> u8 {
        self.sandhi.iter().find(|(p, t, _)| *p == prev && *t == tone).map_or(tone, |(_, _, s)| *s)
    }

    /// The contours and 变音 rules in force, for `PlanOptions::melody`.
    pub fn melody(&self) -> Melody {
        self.melody.clone()
    }

    pub fn unmark(&self, c: char) -> Option<(char, u8)> {
        self.marks.iter().find(|(m, _, _)| *m == c).map(|(_, base, tone)| (*base, *tone))
    }

    /// A letter that belongs to pinyin notation (not a separator, not Han).
    pub fn is_pinyin_char(&self, c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '\'' || c == '’' || c == 'ü' || self.unmark(c).is_some()
    }

    /// One syllable (lowercase, tone mark or trailing digit) → its segments and tone.
    pub fn syllable(&self, s: &str) -> Option<(Vec<Segment>, u8)> {
        let mut tone = self.neutral;
        let mut base = String::new();
        for c in s.chars() {
            if let Some((v, t)) = self.unmark(c) {
                base.push(v);
                tone = t;
            } else if let Some(d) = c.to_digit(10) {
                let d = d as u8;
                tone = if self.toned.contains(&d) { d } else { self.neutral };
            } else {
                base.push(if c == self.v_as { 'ü' } else { c });
            }
        }
        // the onset: longest spelling first (ng before n, as the dialect needs)
        let ini = self.initials.iter().find(|i| base.starts_with(i.key.as_str()));
        let (ini, rest): (Option<&Initial>, &str) = match ini {
            Some(i) => (Some(i), base.get(i.key.len()..).unwrap_or("")),
            None => (None, base.as_str()),
        };
        let parts = if ini.is_none() {
            // y/w spellings first; a bare final ("ian", "u") is the same value without medial letters
            self.finals_zero.get(base.as_str()).or_else(|| self.finals.get(base.as_str()))?
        } else {
            let i = ini.unwrap();
            // j/q/x + u is [y] (ju qu xu = [tɕy tɕʰy ɕy]); z/c/s/r + i is the apical vowel
            let fin = if self.palatal_u.iter().any(|p| p == &i.key) && rest.starts_with('u') {
                format!("ü{}", &rest[1..])
            } else if rest == "i" {
                self.apical_i.iter().find(|(p, _)| p == &i.key).map_or_else(|| rest.to_string(), |(_, k)| k.clone())
            } else {
                rest.to_string()
            };
            self.finals.get(&fin)?
        };
        let mut out = Vec::with_capacity(4);
        if let Some(i) = ini {
            out.push(Segment { aspirated: i.aspirated, ..Segment::new(&i.ipa) });
        }
        for p in parts {
            let nucleus = !self.is_coda(p);
            out.push(if nucleus {
                Segment { tone: Some(tone), stress: Some(if self.toned.contains(&tone) { 1 } else { 0 }), ..Segment::new(p) }
            } else {
                Segment::new(p)
            });
        }
        Some((out, tone))
    }

    /// A word → syllables: split at apostrophes and after tone digits, then by longest match.
    pub fn split_word(&self, w: &str) -> Option<Vec<String>> {
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
                let n = (1..=(cs.len() - i).min(self.max_syllable)).rev().find(|&n| self.syllable(&cs[i..i + n].iter().collect::<String>()).is_some())?;
                out.push(cs[i..i + n].iter().collect::<String>());
                i += n;
            }
            if let Some(last) = out.last_mut() {
                last.push_str(&c[letters.len()..]);
            }
        }
        Some(out)
    }

    /// Whether text is Guiyang notation (or characters): tone digits, tone marks or Han.
    pub fn is_guiyang(&self, text: &str) -> bool {
        text.chars().any(|c| is_han(c) || self.unmark(c).is_some()) || text.split(' ').any(|w| self.split_word(w).is_some())
    }

    /// Segment durations in seconds, from `[timing]`.
    pub fn speech_durations(&self, segs: &mut [Segment], tone: u8, last: bool) {
        let t = &self.timing;
        let toned = self.toned.contains(&tone);
        let coda = segs.last().is_some_and(|x| self.is_coda(&x.ipa));
        let nucleus = segs.iter().position(|x| x.tone.is_some()).unwrap_or(segs.len());
        for (i, x) in segs.iter_mut().enumerate() {
            let ms = if i == nucleus {
                let mut d = if toned { t.vowel } else { t.vowel_neutral };
                if x.ipa.chars().count() > 1 {
                    d += t.diphthong_extra;
                }
                if coda {
                    d -= t.coda_shorten;
                }
                if last { d * t.last_factor } else { d }
            } else if i > nucleus && self.is_coda(&x.ipa) {
                t.coda_nasal
                    * if last { t.coda_last_factor } else { 1.0 }
                    * if toned { 1.0 } else { t.coda_neutral_factor }
            } else if let Some((plain, asp)) = t.phones.get(&x.ipa) {
                if x.aspirated == Some(true) { asp.unwrap_or(*plain) } else { *plain }
            } else if i > 0 && t.glide_phones.iter().any(|g| g == &x.ipa) {
                t.glide
            } else {
                t.default
            };
            x.dur = Some(ms / 1000.0);
        }
    }
}

pub fn is_han(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c) || c == '〇'
}
