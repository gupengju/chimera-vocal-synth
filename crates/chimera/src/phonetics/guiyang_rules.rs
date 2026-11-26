//! The Guiyang notation's rules, parsed from `public/data/guiyang.rules.txt`.
//!
//! The tables used to live in `guiyang.rs` as `match` arms; they are now a text file so a
//! phonologist can edit the dialect's phonology without touching Rust. The same text is
//! embedded here (`include_str!`) as a fallback for a build that ships without `public/`,
//! and `guiyangRules` (wasm) / [`GuiyangG2P::set_rules_text`] replace it at run time.
//!
//! What the file owns: initials (and their aspiration), finals (and how a final expands into
//! phonemes), the y/w spellings of zero-initial syllables, the j/q/x + u and z/c/s/r + i
//! rewrites, the tone-mark letters, the tone-digit alphabet, the digit readings, the phrase
//! pauses, and the per-phoneme durations. Glyph classification (what is a Han character, what
//! is punctuation) stays in code — that is not phonology.

use super::Segment;
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
                        if levels.len() < 2 || !levels.chars().all(|c| ('1'..='5').contains(&c)) {
                            return Err(at(format!("tone {t}: levels are two to five digits 1–5, got {levels:?}")));
                        }
                        r.toned.push(t);
                    }
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
