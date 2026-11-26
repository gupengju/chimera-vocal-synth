import { useEffect, useState } from "react";
import { useStore } from "../store.ts";
import { speakText, speakPhrases, previewText, stopSpeaking, onPlan, currentPlan, spokenWordNow, type SpeakLang } from "../speech.ts";
import { mouthWords } from "../mouthWords.ts";
import { tokenizeIpa } from "../../wasm/engine.ts";
import { Slider, Toggle } from "./Controls.tsx";
import { Chip, Screen } from "./Screen.tsx";
import { ScoreView, planNow } from "./ScoreView.tsx";
import { TubeView } from "./TubeView.tsx";
import { Stroboscope } from "./Stroboscope.tsx";
import type { InfoFn, SlotsFn } from "./AnatomyView.tsx";
import { V } from "../colors.ts";
import { engine } from "../../audio/AudioEngine.ts";

/** The three front ends, picked explicitly — the text is never guessed at. */
const LANGS: { id: SpeakLang; label: string; hint: string }[] = [
  { id: "english", label: "英语", hint: "输入英文，例如 Hello. Can you hear me?" },
  { id: "mandarin", label: "普通话", hint: "输入汉字或拼音，例如 你好。 / ni3 hao3" },
  { id: "guiyang", label: "贵阳话", hint: "输入汉字或记法，例如 国角。 / gue2 jio2" },
];

const EXAMPLES: Record<SpeakLang, string[]> = {
  english: [
    "Hello. Can you hear me?",
    "She sells sea shells by the sea shore.",
    "Every sound here is air moving through a throat.",
    "The quick brown fox jumps over the lazy dog.",
  ],
  mandarin: [
    "你好！今天天气很好。",
    "请问，火车站怎么走？",
    "我喜欢唱歌，也喜欢吃火锅。",
    "好好学习，天天向上。",
    "这个声音是用空气模拟出来的。",
  ],
  guiyang: [
    "明天天气好，去花溪吃牛肉粉。",
    "你搞哪样？",
    "我晓得。",
    "你吃莽莽没有？",
    "我们去赶场。",
    "孃孃，好多钱一斤？",
    "你住在哪点？",
    "他咋个还没得回来？",
    "你莫千翻。",
    "你咋个不张我？",
    "折耳根蛮香。",
  ],
};

// IPA pulmonic consonant chart (rows = manner, columns = place); empty cells are null
const PLACES = ["双唇", "唇齿", "齿", "齿龈", "龈后", "卷舌", "硬腭", "软腭", "小舌", "咽", "声门"];
/** short headings for the chart's narrow columns (the full names are the cells' titles) */
const MANNER_SHORT: Record<string, string> = { 边擦音: "边擦" };
const CHART: [string, (string | null)[][]][] = [
  ["塞音", [["p", "b"], [null, null], [null, null], ["t", "d"], [null, null], ["ʈ", "ɖ"], ["c", "ɟ"], ["k", "g"], ["q", "ɢ"], [null, null], ["ʔ", null]]],
  ["鼻音", [[null, "m"], [null, "ɱ"], [null, null], [null, "n"], [null, null], [null, "ɳ"], [null, "ɲ"], [null, "ŋ"], [null, "ɴ"], [null, null], [null, null]]],
  ["颤音", [[null, "ʙ"], [null, null], [null, null], [null, "r"], [null, null], [null, null], [null, null], [null, null], [null, "ʀ"], [null, null], [null, null]]],
  ["闪音", [[null, null], [null, null], [null, null], [null, "ɾ"], [null, null], [null, "ɽ"], [null, null], [null, null], [null, null], [null, null], [null, null]]],
  ["擦音", [["ɸ", "β"], ["f", "v"], ["θ", "ð"], ["s", "z"], ["ʃ", "ʒ"], ["ʂ", "ʐ"], ["ç", "ʝ"], ["x", "ɣ"], ["χ", "ʁ"], ["ħ", "ʕ"], ["h", "ɦ"]]],
  ["边擦音", [[null, null], [null, null], [null, null], ["ɬ", "ɮ"], [null, null], [null, null], [null, null], [null, null], [null, null], [null, null], [null, null]]],
  ["近音", [[null, null], [null, "ʋ"], [null, null], [null, "ɹ"], [null, null], [null, "ɻ"], [null, "j"], [null, "ɰ"], [null, null], [null, null], [null, null]]],
  ["边音", [[null, null], [null, null], [null, null], [null, "l"], [null, null], [null, "ɭ"], [null, "ʎ"], [null, null], [null, null], [null, null], [null, null]]],
];

/** the segment sounding now in the current utterance */
function soundingSegment(): string | null {
  const sp = currentPlan();
  const now = planNow(sp);
  if (!sp || now < 0) return null;
  return sp.plan.segments.find((s) => now >= s.t0 && now < s.t1)?.ipa ?? null;
}

/** Speak mode's readouts under the chin: the sound being made, jaw, lip shape, soft palate */
/** the word being spoken, at the mouth */
export const speechWords = mouthWords(spokenWordNow, V.white);

export const speechSlots: SlotsFn = (_m, a, f) => {
  const sound = soundingSegment();
  const lips = a.lipAp < 0.05 ? "闭合" : a.lipWidth < 0.55 ? "圆唇" : a.lipAp > 1.3 ? "张开" : "展唇";
  return [
    sound ? { label: "发音", value: sound, color: V.green, ipa: true } : { label: "发音", value: "-", color: V.green },
    { label: "下颌", value: a.jaw.toFixed(1), unit: "cm", color: V.red },
    { label: "唇形", value: lips, color: V.pink },
    { label: "软腭", value: f.velumPort > 0.02 ? "开" : "闭", color: V.white },
  ];
};

/** what the voice is doing: voiced or not, where the noise is, the lungs */
export const speechInfo: InfoFn = (m, _f, tele) => {
  if (!tele) return [];
  const t = tele.t;
  const out = [t.voicing > 0.3 ? `浊音 ${t.f0.toFixed(0)} Hz` : "清音"];
  if (t.noiseLevel > 1 && t.noiseNode >= 0) {
    const s = m.branchS(t.noiseNode, t.phLen, t.orLen);
    if (s >= 0) out.push(`噪声在 ${s.toFixed(1)} cm`);
  }
  out.push(`肺压 ${t.pLung.toFixed(1)} cmH₂O`);
  return out;
};

function ConsonantChart(props: { onPad: (c: string) => void }) {
  const [now, setNow] = useState<string | null>(null);
  useEffect(() => {
    const id = setInterval(() => setNow(engine.latest ? soundingSegment() : null), 60);
    return () => clearInterval(id);
  }, []);
  return (
    <div className="chart-scroll">
      <table className="ipa-chart">
        <thead>
          <tr>
            <th />
            {PLACES.map((p) => (
              <th key={p} scope="col" title={p} aria-label={p}>
                {p}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {CHART.map(([manner, cells]) => (
            <tr key={manner}>
              <th scope="row" title={manner} aria-label={manner}>
                {MANNER_SHORT[manner] ?? manner}
              </th>
              {cells.map((pair, i) => (
                <td key={i}>
                  {pair.map((sym, j) =>
                    sym ? (
                      <button key={j} className={`ipa${sym === now ? " now" : ""}`} onClick={() => props.onPad(sym)} title={`在两个元音之间读它`}>
                        {sym}
                      </button>
                    ) : (
                      <span key={j} className="ipa-empty" />
                    ),
                  )}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Speak mode's right column: what to say, the gestural score, the consonant chart, the tube and folds. */
export function SpeakRight() {
  const body = useStore((s) => s.body);
  const f0 = useStore((s) => s.source.f0);
  const start = useStore((s) => s.start);
  const [lang, setLang] = useState<SpeakLang>("english");
  const [text, setText] = useState(EXAMPLES.english[0]);
  const [ipa, setIpa] = useState(false);
  const [rate, setRate] = useState(1);
  const [busy, setBusy] = useState(false);
  const [trans, setTrans] = useState(currentPlan()?.transcription ?? "");
  const [vowel, setVowel] = useState("ɑ");
  useEffect(() => onPlan((p) => setTrans(p?.transcription ?? "")), []);

  const speak = async () => {
    setBusy(true);
    try {
      await start();
      await speakText(text, { ipa, lang, body, rate, f0 });
    } finally {
      setBusy(false);
    }
  };
  /** Switching language swaps only an untouched example; typed text is left alone. */
  const pickLang = (id: SpeakLang) => {
    setLang(id);
    const swap = EXAMPLES[lang].includes(text) || text === "";
    const next = swap ? EXAMPLES[id][0] : text;
    if (swap) setText(next);
    // the front end changed, so the score is stale even when the text did not move
    void previewText(next, { ipa, lang: id, body, rate, f0 });
  };
  const pad = async (c: string) => {
    await start();
    speakPhrases([{ segments: tokenizeIpa(`ˈ${vowel}${c}${vowel}`), intonation: "flat" }], { body, rate, f0 });
  };

  return (
    <div className="right right-speak">
      <Screen
        className="s-say"
        title="朗读"
        sub="文本转语音"
        tools={
          <>
            <div className="lang-pick" role="group" aria-label="语言">
              {LANGS.map((l) => (
                <Chip key={l.id} on={!ipa && lang === l.id} onClick={() => pickLang(l.id)} disabled={ipa}>
                  {l.label}
                </Chip>
              ))}
            </div>
            {
              /* the IPA toggle is commented out: text is always read as words, `ipa` stays false
            <Toggle label="文本是国际音标" checked={ipa} onChange={setIpa} />
            */
            }
          </>
        }
      >
        <div className="say">
          <div className="say-row">
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) void speak();
                e.stopPropagation();
              }}
              rows={2}
              spellCheck={!ipa}
              aria-label={ipa ? "要读的国际音标" : "要朗读的文字"}
              placeholder={ipa ? "输入国际音标，例如 ˈhɛloʊ" : (LANGS.find((l) => l.id === lang)?.hint ?? "")}
            />
            <div className="say-actions">
              <button className="primary" onClick={() => void speak()} disabled={busy}>
                {busy ? "准备中…" : "朗读"}
              </button>
              <button className="ghost" onClick={stopSpeaking}>
                停止
              </button>
            </div>
          </div>
          <div className="say-row">
            <p className="transcription" aria-live="polite">
              {trans ? `/${trans}/` : "转写会显示在这里。"}
            </p>
            <div className="say-rate">
              <Slider label="语速" value={rate} min={0.5} max={1.8} onChange={setRate} format={(v) => `${v.toFixed(2)}×`} />
            </div>
          </div>
          <div className="examples" role="group" aria-label="例句">
            <label className="example-pick">
              <span className="example-pick-label">例句</span>
              <select
                aria-label="选一个例句"
                value=""
                onChange={(e) => {
                  const ex = e.target.value;
                  if (!ex) return;
                  setText(ex);
                  setIpa(false);
                  // picking an example lays it out on the score at once; 朗读 is for the sound
                  void previewText(ex, { ipa: false, lang, body, rate, f0 });
                }}
              >
                <option value="" disabled>
                  选一个例句…
                </option>
                {EXAMPLES[lang].map((ex) => (
                  <option key={ex} value={ex}>
                    {ex}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </div>
      </Screen>
      <Screen className="s-score" title="姿态谱">
        <ScoreView />
      </Screen>
      <Screen
        className="s-chart"
        title="辅音"
        sub="夹在两个元音之间"
        tools={
          <div className="vowel-pick" role="group" aria-label="周围的元音">
            {["ɑ", "i", "u", "ə"].map((v) => (
              <Chip key={v} on={vowel === v} onClick={() => setVowel(v)} className="ipa-chip">
                {v}
              </Chip>
            ))}
          </div>
        }
      >
        <ConsonantChart onPad={(c) => void pad(c)} />
      </Screen>
      <Screen className="s-tube" title="声道管" sub="从声门到唇">
        <TubeView zh />
      </Screen>
      <Screen className="s-folds" title="声带" sub="慢放 100 倍">
        <Stroboscope zh />
      </Screen>
    </div>
  );
}
