/**
 * The Guiyang wordlist ships gzipped (91 KB of text in 43 KB): `public/data/guiyang.txt` is the
 * editable source, `public/data/guiyang.txt.gz` the copy the page and the CLI read.
 *
 * `bun run dev` and `bun run build` run this first, so an edit to the .txt takes effect on the
 * next start. Nothing is written when the gzip already holds the same text (no diff churn).
 *
 * Usage: bun tools/gzip-data.ts
 */
import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { gunzipSync, gzipSync } from "node:zlib";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const SRC = join(ROOT, "public/data/guiyang.txt");
const GZ = join(ROOT, "public/data/guiyang.txt.gz");

// line endings are canonicalised: a Windows editor saves CRLF, the packed table stays LF
const text = readFileSync(SRC, "utf8").replace(/\r\n?/g, "\n");
const same = existsSync(GZ) && gunzipSync(readFileSync(GZ)).toString("utf8") === text;
if (same) {
  console.log(`guiyang.txt.gz is up to date (${text.length} chars)`);
} else {
  const packed = gzipSync(text, { level: 9 });
  packed.fill(0, 4, 8); // the header's mtime would otherwise differ on every run
  writeFileSync(GZ, packed);
  console.log(`${text.length} chars → guiyang.txt.gz (${statSync(GZ).size} bytes)`);
}
