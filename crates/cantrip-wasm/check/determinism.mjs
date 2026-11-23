// The engine in the browser answers as the native host does, byte for byte (docs/engine-in-the-browser.md): every
// scenario is dealt to both - cantrip-native-host over its stdin, and cantrip_wasm.wasm in this process, as a page loads
// it - with no seat driven, so the script plays the whole game; each gets the same requests (deal, start, each seat's
// view, close), and every reply must be the same. A realm duel is played in the browser and verified on the server by
// replaying it on the native engine, which holds only if the two never differ.
//
//   node crates/cantrip-wasm/check/determinism.mjs <cantrip_wasm.wasm> <cantrip-native-host> <scenarios dir> [from-to]
import { readFileSync, readdirSync } from "node:fs";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { join } from "node:path";

const [wasmPath, hostPath, dir, range] = process.argv.slice(2);

// The page's side of the module: write a request into memory it lends, call, read the reply.
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
const wasm = instance.exports;
const encoder = new TextEncoder();
const decoder = new TextDecoder();
function inBrowser(line) {
  const bytes = encoder.encode(line);
  const at = wasm.cantrip_alloc(bytes.length);
  new Uint8Array(wasm.memory.buffer, at, bytes.length).set(bytes);
  const length = wasm.cantrip_call(at, bytes.length);
  wasm.cantrip_free(at, bytes.length);
  return decoder.decode(new Uint8Array(wasm.memory.buffer, wasm.cantrip_reply(), length));
}

const host = spawn(hostPath, [], { stdio: ["pipe", "pipe", "ignore"] });
const replies = createInterface({ input: host.stdout })[Symbol.asyncIterator]();
async function native(line) {
  host.stdin.write(line + "\n");
  return (await replies.next()).value;
}

let files = readdirSync(dir).filter((f) => f.endsWith(".cantrip")).sort();
if (range) {
  const [from, to] = range.split("-").map(Number);
  files = files.slice(from, to);
}

let same = 0;
let gameId = 0;
const differ = [];
for (const file of files) {
  const text = readFileSync(join(dir, file), "utf8");
  const seats = Math.max(2, ...[...text.matchAll(/^\[Bot(\d) Deck\]/gm)].map((m) => Number(m[1])));
  const requests = [JSON.stringify({ Cmd: "NewGame", Seats: Array.from({ length: seats }, (_, i) => ({ Name: `Bot${i + 1}`, Deck: [] })), Seed: 1, Scenario: { Name: file, Text: text, Driven: [] } })];
  const first = await native(requests[0]);
  const firstWasm = inBrowser(requests[0]);
  let ok = first === firstWasm;
  if (ok && first.includes('"GameId"')) {
    gameId++;
    const more = [{ Cmd: "Start", GameId: gameId }, ...Array.from({ length: seats }, (_, i) => ({ Cmd: "View", GameId: gameId, Seat: i + 1 })), { Cmd: "Close", GameId: gameId }];
    for (const request of more.map((r) => JSON.stringify(r))) {
      if ((await native(request)) !== inBrowser(request)) {
        ok = false;
        differ.push(`${file}: ${request}`);
        break;
      }
    }
  } else if (!ok) {
    differ.push(`${file}: the deal`);
  }
  if (ok) same++;
}
host.stdin.end();
console.log(JSON.stringify({ scenarios: files.length, same, differ: differ.length }));
for (const line of differ.slice(0, 30)) console.log(line);
process.exitCode = differ.length === 0 ? 0 : 1;
