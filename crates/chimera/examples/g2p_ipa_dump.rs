//! 文本 → IPA 音段流的诊断导出（与 C++ tools/g2p_ipa_dump.cpp 输出同一格式）。
//! 只用于 tools/g2p_ipa_diff.py 的两端逐条比对，不参与渲染。
//!
//!   cargo run --release --example g2p_ipa_dump -- mandarin ../frft-vocal-synth/bin/data/data_mandarin.txt -
use std::io::{BufRead, Read, Write};

fn shown(phrases: &[chimera::phonetics::english::Phrase]) -> String {
    use chimera::phonetics::Intonation;
    phrases
        .iter()
        .map(|p| {
            let segs: Vec<String> = p
                .segments
                .iter()
                .map(|s| {
                    format!(
                        "{}{}{}{}{}",
                        if s.stress == Some(1) && s.tone.is_none() { "ˈ" } else { "" },
                        s.ipa,
                        if s.aspirated == Some(true) { "ʰ" } else { "" },
                        s.tone.map_or(String::new(), |t| t.to_string()),
                        if s.word_end == Some(true) { "·" } else { "" }
                    )
                })
                .collect();
            segs.join(",") + if p.intonation == Intonation::Question { "?" } else { "" }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: g2p_ipa_dump <mandarin|guiyang> <dict> [text]");
        std::process::exit(2);
    }
    let lang = args[1].as_str();
    let mut d = String::new();
    std::fs::File::open(&args[2])
        .and_then(|mut f| f.read_to_string(&mut d))
        .unwrap_or_else(|e| {
            eprintln!("{}: {e}", args[2]);
            std::process::exit(2)
        });
    let mandarin = match lang {
        "mandarin" => true,
        "guiyang" => false,
        l => {
            eprintln!("unknown language {l:?}; have mandarin, guiyang");
            std::process::exit(2)
        }
    };
    let mut zh = chimera::phonetics::mandarin::MandarinG2P::new();
    let mut gy = chimera::phonetics::guiyang::GuiyangG2P::new();
    if mandarin {
        zh.load_dict(&d);
    } else {
        gy.load_dict(&d);
    }
    let mut texts: Vec<String> = Vec::new();
    match args.get(3).map(String::as_str) {
        Some("-") | None => {
            for line in std::io::stdin().lock().lines() {
                texts.push(line.unwrap());
            }
        }
        Some(t) => texts.push(t.to_string()),
    }
    let mut out = std::io::stdout().lock();
    for t in &texts {
        let s = if mandarin { shown(&zh.to_phrases(t)) } else { shown(&gy.to_phrases(t)) };
        writeln!(out, "{s}").unwrap();
    }
}
