// The phase-0 spike on onnxruntime-web's WASM backend, single-threaded as a page without COOP/COEP runs it: parity
// against PyTorch's reference outputs and positions a second per bucket.
//
//   node run.mjs [--out DIR] [--seconds S]
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as ort from 'onnxruntime-web';

const TOKENS = [32, 64, 128];
const BATCHES = [1, 16, 64, 512];
const VALUE_TOLERANCE = 1e-3;
const SAME_ARGMAX = 0.995;

const arg = (name, fallback) => {
  const i = process.argv.indexOf(name);
  return i < 0 ? fallback : process.argv[i + 1];
};
const out = arg('--out', join(dirname(fileURLToPath(import.meta.url)), '..', 'out'));
const seconds = Number(arg('--seconds', 1));

ort.env.wasm.numThreads = 1;
ort.env.wasm.simd = true;

const raw = (dir, name, Type) => {
  const b = readFileSync(join(dir, name));
  return new Type(b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength));
};

function reference(tokens) {
  const dir = join(out, `ref-t${tokens}`);
  const value = raw(dir, 'value.f32', Float32Array);
  const x = raw(dir, 'features.f32', Float32Array);
  const n = value.length;
  return {
    tokens, n, features: x.length / (n * tokens), x, value,
    kind: raw(dir, 'kind.i64', BigInt64Array),
    present: raw(dir, 'present.u8', Uint8Array),
    option: raw(dir, 'option.u8', Uint8Array),
    policy: raw(dir, 'policy.f32', Float32Array),
  };
}

function batch(r, at, size) {
  const rows = Array.from({ length: size }, (_, k) => (at + k) % r.n);
  const take = (src, width, Type) => {
    const dst = new Type(size * width);
    rows.forEach((row, k) => dst.set(src.subarray(row * width, (row + 1) * width), k * width));
    return dst;
  };
  const t = r.tokens;
  return {
    rows,
    feeds: {
      features: new ort.Tensor('float32', take(r.x, t * r.features, Float32Array), [size, t, r.features]),
      kind: new ort.Tensor('int64', take(r.kind, t, BigInt64Array), [size, t]),
      present: new ort.Tensor('bool', take(r.present, t, Uint8Array), [size, t]),
      option: new ort.Tensor('bool', take(r.option, t, Uint8Array), [size, t]),
    },
  };
}

const argmax = (a) => a.reduce((best, v, i) => (v > a[best] ? i : best), 0);

async function parity(session, r, size) {
  let worst = 0, same = 0;
  for (let at = 0; at < r.n; at += size) {
    const b = batch(r, at, Math.min(size, r.n - at));
    const { value, policy } = await session.run(b.feeds);
    const t = r.tokens;
    b.rows.forEach((row, k) => {
      worst = Math.max(worst, Math.abs(value.data[k] - r.value[row]));
      if (argmax(policy.data.subarray(k * t, (k + 1) * t)) === argmax(r.policy.subarray(row * t, (row + 1) * t))) same++;
    });
  }
  return [worst, same / r.n];
}

async function throughput(session, r, size) {
  const b = batch(r, 0, size);
  await session.run(b.feeds);
  let runs = 0;
  const start = performance.now();
  while (performance.now() - start < seconds * 1000) {
    await session.run(b.feeds);
    runs++;
  }
  return (runs * size) / ((performance.now() - start) / 1000);
}

console.log(`onnxruntime-web ${ort.env.versions?.web ?? ''} wasm, 1 thread; parity against PyTorch (value within ${VALUE_TOLERANCE}, same argmax on ${SAME_ARGMAX * 100}%+), positions/s:`);
let failed = false;
for (const tokens of TOKENS) {
  const r = reference(tokens);
  const session = await ort.InferenceSession.create(readFileSync(join(out, `spike-t${tokens}.onnx`)), {
    executionProviders: ['wasm'],
    graphOptimizationLevel: 'all',
  });
  for (const size of BATCHES) {
    try {
      const [worst, same] = await parity(session, r, size);
      const rate = await throughput(session, r, size);
      const ok = worst <= VALUE_TOLERANCE && same >= SAME_ARGMAX;
      failed ||= !ok;
      console.log(`  wasm, 1 thread  T${String(tokens).padEnd(3)} B${String(size).padEnd(3)}  value ${worst.toExponential(1)}  argmax ${(same * 100).toFixed(2).padStart(6)}%  ${ok ? 'ok  ' : 'FAIL'}  ${rate.toFixed(0).padStart(9)}/s`);
    } catch (e) {
      failed = true;
      console.log(`  wasm, 1 thread  T${tokens} B${size}  error: ${e.message}`);
    }
  }
}
process.exit(failed ? 1 : 0);
