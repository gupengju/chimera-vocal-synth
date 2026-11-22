/**
 * Guiyang (贵阳话) through the wasm build: the book's notation and characters through the
 * dictionary, its 55/21/42/13 tone contours in the plan, and the explicit three-language pick.
 */
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { initEngine } from "./harness.ts";

await initEngine();
const { GuiyangG2P, isGuiyang, isMandarin, planPhrases } = await import("../src/wasm/engine.ts");
const { transcribe } = await import("../src/ui/words.ts");
const gy = new GuiyangG2P();
gy.setDictionaryText(gunzipSync(readFileSync(new URL("../public/data/guiyang.txt.gz", import.meta.url))).toString());
const male = { female: 0, child: 0 };
const tones = (text: string) => gy.toPhrases(text).map((p) => p.segments.flatMap((s) => (s.tone ? [s.tone] : [])));
const pitch = (text: string, toneScheme?: "mandarin" | "guiyang") =>
  planPhrases(gy.toPhrases(text), { body: male, t0: 0, f0: 110, toneScheme })
    .gestures.filter((g) => g.label === "pitch")
    .sort((a, b) => a.t0 - b.t0)
    .map((g) => g.targets.f0!);

describe("guiyang", () => {
  test("the notation's own rhymes: io, iw, ue, and ɛ for e", () => {
    expect(gy.toPhrases("gue2 jio2")[0].segments.map((s) => s.ipa + (s.aspirated ? "ʰ" : ""))).toEqual(["k", "uɛ", "tɕ", "io"]);
    expect(tones("gue2 jio2")).toEqual([[2, 2]]);
    // iw is [iu], iu is [iəu] (not Mandarin's iou)
    expect(gy.toPhrases("qiw2")[0].segments.map((s) => s.ipa)).toEqual(["tɕ", "iu"]);
    expect(gy.toPhrases("diu1")[0].segments.map((s) => s.ipa)).toEqual(["t", "iəu"]);
    // e is [ɛ]; the book's z/c/s are alveolar [ts], r is [z]
    expect(gy.toPhrases("be2")[0].segments.map((s) => s.ipa)).toEqual(["p", "ɛ"]);
    expect(gy.toPhrases("zi1")[0].segments.map((s) => s.ipa)).toEqual(["ts", "ɨ"]);
    expect(gy.toPhrases("ri2")[0].segments.map((s) => s.ipa)).toEqual(["z", "ɨ"]);
  });

  test("characters read through the dictionary", () => {
    expect(tones("国角。")).toEqual([[2, 2]]);
    expect(gy.toPhrases("国角。")[0].intonation).toBe("statement");
    expect(gy.toPhrases("国角？")[0].intonation).toBe("question");
  });

  test("tone 2 falls in Guiyang and rises in Mandarin", () => {
    expect(pitch("gue2", "guiyang")[1] < pitch("gue2", "guiyang")[0]).toBe(true); // 21
    expect(pitch("gue2", "mandarin")[1] > pitch("gue2", "mandarin")[0]).toBe(true); // 35
    // tone 4 is 13 (rising), tone 3 is 42 (falling)
    expect(pitch("gue4", "guiyang")[1] > pitch("gue4", "guiyang")[0]).toBe(true);
    expect(pitch("gue3", "guiyang")[1] < pitch("gue3", "guiyang")[0]).toBe(true);
  });

  test("the transcription uses the dialect\'s tone letters", () => {
    expect(transcribe(gy.toPhrases("国角。"), "guiyang")).toBe("kuɛ˨˩ ko˨˩");
    expect(transcribe(gy.toPhrases("国角。"))).toBe("kuɛ˧˥ ko˧˥"); // Mandarin letters, for contrast
  });

  test("the page's language pick is explicit: only the dialect's own spellings are guiyang", () => {
    expect(isGuiyang("gue2 jio2")).toBe(true);
    expect(isGuiyang("国角。")).toBe(true);
    expect(isGuiyang("qiw2")).toBe(true);
    expect(isGuiyang("Hello there")).toBe(false);
    // Mandarin's detector is deliberately permissive (any tone digit or mark), so the page
    // never guesses between the two: the language chips decide.
    expect(isMandarin("gue2 jio2")).toBe(true);
  });
});
