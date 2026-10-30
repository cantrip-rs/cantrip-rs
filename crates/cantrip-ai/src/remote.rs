//! Evaluating on an inference server (docs/ai-player-plan.md, "Decisions": GPU inference for self-play and server
//! seats): `RemoteEvaluator` sends each batch to `ai/cantrip_ai/serve.py`, which gathers the rows of every connection
//! into GPU batches per token bucket and answers each request with its rows' outputs.
//!
//! The transport is TCP (`127.0.0.1:PORT` on one machine, on Windows and Linux alike) with length-prefixed binary frames,
//! all little-endian: a frame is a u32 byte count and that many bytes.
//!
//! - **Hello** (client): `MAGIC`, u32 `PROTOCOL`, u32 the encoding schema, u64 the encoder hash, u64 the model's
//!   identity (FNV-1a 64 of its `manifest.json`), u16 and the model's name in UTF-8 (for the server's log).
//! - **Welcome** (server): u8 0, u16 and a description of the server (its device); or u8 1, u16 and why it refuses
//!   (another protocol, schema or encoder hash, or no model with that manifest). A refused connection is closed.
//! - **Evaluate** (client): u8 `EVALUATE`, u32 rows, u8 the card width (2 or 4 bytes), u8 the numbers' form
//!   (`NUMS_SPARSE` or `NUMS_DENSE`), u8 each row's real tokens; then, over the real tokens of every row in order (N in
//!   all): kind u8 [N], card [N] (the model's rows, already mapped from the runtime's), cats u8 [N, CATS], ptrs u8
//!   [N, PTRS], the legal bits [ceil(N / 8)] (bit `i % 8` of byte `i / 8`); and the numbers: sparse, each token's count
//!   of non-zero numbers u8 [N], then their features u8 [K] and values f16 [K], token by token in feature order (K the
//!   counts' sum); or dense, f32 [N, NUMS], which is sent only when some number is not exact in f16 (an encoded
//!   position's always are). The sparse form is what the server unpacks with the fewest steps: a position has about 6
//!   non-zero numbers a token of 96.
//! - **Outputs** (server): u8 0, then f32 value [rows], f32 aux [rows, 2], and f32 policy logits at the legal tokens
//!   only, in the order the legal bits were sent; or u8 1, u16 and an error, after which the server closes.
//!
//! A row's real tokens are those up to its last one that is not padding: everything past is padding, which the server
//! fills in. The server evaluates each row at the smallest token bucket that holds it, padded to a fixed batch size,
//! with deterministic kernels, so a row's outputs never depend on what it was batched with (the local evaluators run a
//! batch at its largest row's bucket). Every token that is not a legal option has the logit `MASKED` in every backend,
//! so the policy is rebuilt from the legal logits exactly.
//!
//! The client keeps a pool of connections, so the threads sharing an evaluator (the AI host's workers) each have one
//! while they evaluate; a connection is blocking, and the server answers its requests in order.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use cantrip_obs::encode::{
    BUCKETS, CATS, MAX_TOKENS, NO_POINTER, NUMS, PTRS, SCHEMA_VERSION, UNKNOWN_CARD, encoder_hash,
    f16_bits, f16_value, fnv1a64,
};

use crate::CardIndex;
use crate::eval::{EvalError, Evaluator, FeatureBatch, MASKED, Model, Outputs, outputs};

pub const MAGIC: &[u8; 8] = b"CNTRPAI\0";
/// The wire protocol's version: the server refuses a client of another.
pub const PROTOCOL: u32 = 1;
pub const EVALUATE: u8 = 1;
pub const NUMS_SPARSE: u8 = 1;
pub const NUMS_DENSE: u8 = 2;
/// The largest frame either side accepts.
pub const MAX_FRAME: usize = 1 << 28;
/// How long a connection waits for the server before the evaluation is an error.
const TIMEOUT: Duration = Duration::from_secs(300);

fn err(e: impl std::fmt::Display) -> EvalError {
    EvalError(e.to_string())
}

/// A model directory's identity on the wire: FNV-1a 64 of its `manifest.json`, which export writes once per model.
pub fn model_identity(dir: &Path) -> Result<u64, EvalError> {
    let path = dir.join("manifest.json");
    let bytes = std::fs::read(&path).map_err(|e| err(format!("{}: {e}", path.display())))?;
    Ok(fnv1a64(&bytes))
}

/// The hello frame's payload for a model.
pub fn hello(identity: u64, name: &str) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    out.extend(PROTOCOL.to_le_bytes());
    out.extend(SCHEMA_VERSION.to_le_bytes());
    out.extend(encoder_hash().to_le_bytes());
    out.extend(identity.to_le_bytes());
    let name = &name.as_bytes()[..name.len().min(u16::MAX as usize)];
    out.extend((name.len() as u16).to_le_bytes());
    out.extend(name);
    out
}

/// A hello read back: (protocol, schema, encoder hash, model identity, name).
pub fn read_hello(b: &[u8]) -> Result<(u32, u32, u64, u64, String), EvalError> {
    let mut r = Reader::new(b);
    if r.take(8)? != MAGIC {
        return Err(err("not a Cantrip inference client"));
    }
    let protocol = r.u32()?;
    let schema = r.u32()?;
    let hash = r.u64()?;
    let identity = r.u64()?;
    let n = r.u16()? as usize;
    let name = String::from_utf8_lossy(r.take(n)?).into_owned();
    r.end()?;
    Ok((protocol, schema, hash, identity, name))
}

/// A status frame: 0 and a text (a welcome), or 1 and an error.
pub fn status(ok: bool, text: &str) -> Vec<u8> {
    let text = &text.as_bytes()[..text.len().min(u16::MAX as usize)];
    let mut out = vec![if ok { 0 } else { 1 }];
    out.extend((text.len() as u16).to_le_bytes());
    out.extend(text);
    out
}

fn read_status(b: &[u8]) -> Result<Result<String, String>, EvalError> {
    let mut r = Reader::new(b);
    let ok = r.u8()?;
    let n = r.u16()? as usize;
    let text = String::from_utf8_lossy(r.take(n)?).into_owned();
    Ok(if ok == 0 { Ok(text) } else { Err(text) })
}

/// Whether token `at` of a batch is all padding.
fn padding(b: &FeatureBatch, at: usize) -> bool {
    b.kind[at] == 0
        && b.card[at] == UNKNOWN_CARD as i64
        && !b.legal[at]
        && b.cats[at * CATS..(at + 1) * CATS].iter().all(|&c| c == 0)
        && b.ptrs[at * PTRS..(at + 1) * PTRS]
            .iter()
            .all(|&p| p == NO_POINTER as i64)
        && b.nums[at * NUMS..(at + 1) * NUMS]
            .iter()
            .all(|x| x.to_bits() == 0)
}

fn byte(v: i64, what: &str) -> Result<u8, EvalError> {
    u8::try_from(v).map_err(|_| err(format!("{what} {v} does not fit the wire's byte")))
}

/// The evaluate frame's payload for a batch, its card rows mapped through `remap` (empty: unmapped). A batch no encoder
/// made - a kind, categorical or pointer outside a byte, a negative card - is an error, never a different value.
///
/// The rows go in the order of the smallest bucket that holds each (stable), so each bucket's rows are one run of the
/// frame, which the server takes without regrouping; the order is returned for `read_reply`.
pub fn request(b: &FeatureBatch, remap: &[u32]) -> Result<(Vec<u8>, Vec<usize>), EvalError> {
    let t = b.tokens;
    if t > MAX_TOKENS {
        return Err(err(format!("{t} tokens")));
    }
    let rows = b.rows;
    let lengths: Vec<usize> = (0..rows)
        .map(|i| {
            (0..t)
                .rev()
                .find(|&k| !padding(b, i * t + k))
                .map_or(0, |k| k + 1)
        })
        .collect();
    let bucket = |l: usize| {
        BUCKETS
            .iter()
            .position(|&b| b >= l.max(1))
            .unwrap_or(BUCKETS.len())
    };
    let mut order: Vec<usize> = (0..rows).collect();
    order.sort_by_key(|&i| bucket(lengths[i]));
    let real: Vec<usize> = order
        .iter()
        .flat_map(|&i| (0..lengths[i]).map(move |k| i * t + k))
        .collect();
    let n = real.len();
    let mut cards = Vec::with_capacity(n);
    for &at in &real {
        let c = u32::try_from(b.card[at])
            .map_err(|_| err(format!("a card row {} outside the index", b.card[at])))?;
        cards.push(if remap.is_empty() {
            c
        } else {
            remap.get(c as usize).copied().unwrap_or(UNKNOWN_CARD)
        });
    }
    let wide = cards.iter().any(|&c| c > u16::MAX as u32);
    let exact = real.iter().all(|&at| {
        b.nums[at * NUMS..(at + 1) * NUMS]
            .iter()
            .all(|&x| f16_value(f16_bits(x)).to_bits() == x.to_bits())
    });
    let mut out = Vec::with_capacity(16 + rows + n * (24 + CATS + PTRS));
    out.push(EVALUATE);
    out.extend((rows as u32).to_le_bytes());
    out.push(if wide { 4 } else { 2 });
    out.push(if exact { NUMS_SPARSE } else { NUMS_DENSE });
    out.extend(order.iter().map(|&i| lengths[i] as u8));
    for &at in &real {
        out.push(byte(b.kind[at], "a kind")?);
    }
    for &c in &cards {
        if wide {
            out.extend(c.to_le_bytes());
        } else {
            out.extend((c as u16).to_le_bytes());
        }
    }
    for &at in &real {
        for &c in &b.cats[at * CATS..(at + 1) * CATS] {
            out.push(byte(c, "a categorical")?);
        }
    }
    for &at in &real {
        for &p in &b.ptrs[at * PTRS..(at + 1) * PTRS] {
            out.push(byte(p, "a pointer")?);
        }
    }
    let mut bits = vec![0u8; n.div_ceil(8)];
    for (i, &at) in real.iter().enumerate() {
        if b.legal[at] {
            bits[i / 8] |= 1 << (i % 8);
        }
    }
    out.extend(bits);
    if exact {
        let mut features = Vec::new();
        let mut values = Vec::new();
        for &at in &real {
            let mut count = 0u8;
            for (k, &x) in b.nums[at * NUMS..(at + 1) * NUMS].iter().enumerate() {
                if x.to_bits() != 0 {
                    count += 1;
                    features.push(k as u8);
                    values.extend(f16_bits(x).to_le_bytes());
                }
            }
            out.push(count);
        }
        out.extend(features);
        out.extend(values);
    } else {
        for &at in &real {
            for &x in &b.nums[at * NUMS..(at + 1) * NUMS] {
                out.extend(x.to_le_bytes());
            }
        }
    }
    Ok((out, order))
}

/// An evaluate payload read back as a batch of `MAX_TOKENS`-wide rows, in the frame's order (what a server is sent;
/// the Rust test server evaluates it with a local backend).
pub fn read_request(bytes: &[u8]) -> Result<FeatureBatch, EvalError> {
    let mut r = Reader::new(bytes);
    if r.u8()? != EVALUATE {
        return Err(err("not an evaluate frame"));
    }
    let rows = r.u32()? as usize;
    let width = r.u8()? as usize;
    let form = r.u8()?;
    if width != 2 && width != 4 {
        return Err(err(format!("a card width of {width}")));
    }
    let lengths: Vec<usize> = r.take(rows)?.iter().map(|&l| l as usize).collect();
    if lengths.iter().any(|&l| l > MAX_TOKENS) {
        return Err(err("a row longer than the largest bucket"));
    }
    let n: usize = lengths.iter().sum();
    let t = MAX_TOKENS;
    let mut b = FeatureBatch {
        rows,
        tokens: t,
        kind: vec![0; rows * t],
        card: vec![UNKNOWN_CARD as i64; rows * t],
        cats: vec![0; rows * t * CATS],
        ptrs: vec![NO_POINTER as i64; rows * t * PTRS],
        nums: vec![0.0; rows * t * NUMS],
        legal: vec![false; rows * t],
        options: vec![(0, 0); rows],
    };
    let at: Vec<usize> = (0..rows)
        .flat_map(|i| (0..lengths[i]).map(move |k| i * t + k))
        .collect();
    let kinds = r.take(n)?;
    let cards = r.take(n * width)?;
    let cats = r.take(n * CATS)?;
    let ptrs = r.take(n * PTRS)?;
    let bits = r.take(n.div_ceil(8))?;
    for (i, &a) in at.iter().enumerate() {
        b.kind[a] = kinds[i] as i64;
        b.card[a] = if width == 2 {
            u16::from_le_bytes([cards[2 * i], cards[2 * i + 1]]) as i64
        } else {
            u32::from_le_bytes(cards[4 * i..4 * i + 4].try_into().unwrap()) as i64
        };
        for c in 0..CATS {
            b.cats[a * CATS + c] = cats[i * CATS + c] as i64;
        }
        for p in 0..PTRS {
            b.ptrs[a * PTRS + p] = ptrs[i * PTRS + p] as i64;
        }
        b.legal[a] = bits[i / 8] & (1 << (i % 8)) != 0;
    }
    match form {
        NUMS_SPARSE => {
            let counts = r.take(n)?;
            let k: usize = counts.iter().map(|&c| c as usize).sum();
            let features = r.take(k)?;
            let values = r.take(2 * k)?;
            let mut j = 0;
            for (i, &a) in at.iter().enumerate() {
                for _ in 0..counts[i] {
                    let f = features[j] as usize;
                    if f >= NUMS {
                        return Err(err(format!("a number's feature {f}")));
                    }
                    b.nums[a * NUMS + f] =
                        f16_value(u16::from_le_bytes([values[2 * j], values[2 * j + 1]]));
                    j += 1;
                }
            }
        }
        NUMS_DENSE => {
            for &a in &at {
                for k in 0..NUMS {
                    b.nums[a * NUMS + k] = r.f32()?;
                }
            }
        }
        other => return Err(err(format!("a numbers form {other}"))),
    }
    r.end()?;
    Ok(b)
}

/// The outputs frame's payload: a status of 0, value, aux and the logits at the legal tokens, as a server writes it from
/// `outputs` of a batch `read_request` made.
pub fn reply(b: &FeatureBatch, out: &Outputs, policy: &[f32]) -> Vec<u8> {
    let mut v = vec![0u8];
    for x in &out.value {
        v.extend(x.to_le_bytes());
    }
    for a in &out.aux {
        v.extend(a[0].to_le_bytes());
        v.extend(a[1].to_le_bytes());
    }
    for (at, &l) in b.legal.iter().enumerate() {
        if l {
            v.extend(policy[at].to_le_bytes());
        }
    }
    v
}

/// A reply to `batch`'s request (its rows sent in `order`) as the outputs a local evaluator gives.
pub fn read_reply(
    batch: &FeatureBatch,
    order: &[usize],
    bytes: &[u8],
) -> Result<Outputs, EvalError> {
    if bytes.first() != Some(&0) {
        return Err(err(format!(
            "the inference server: {}",
            read_status(bytes)?.err().unwrap_or_default()
        )));
    }
    let mut r = Reader::new(&bytes[1..]);
    let (rows, t) = (batch.rows, batch.tokens);
    if order.len() != rows {
        return Err(err("a reply to another batch"));
    }
    let mut value = vec![0.0; rows];
    let mut aux = vec![0.0; 2 * rows];
    for &i in order {
        value[i] = r.f32()?;
    }
    for &i in order {
        aux[2 * i] = r.f32()?;
        aux[2 * i + 1] = r.f32()?;
    }
    let mut policy = vec![MASKED; rows * t];
    for &i in order {
        for at in i * t..(i + 1) * t {
            if batch.legal[at] {
                policy[at] = r.f32()?;
            }
        }
    }
    r.end()?;
    Ok(outputs(batch, &value, &policy, &aux))
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Reader { b, at: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], EvalError> {
        let s = self
            .b
            .get(self.at..self.at.saturating_add(n))
            .ok_or_else(|| err("a frame cut short"))?;
        self.at += n;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, EvalError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, EvalError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, EvalError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, EvalError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32, EvalError> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn end(&self) -> Result<(), EvalError> {
        if self.at == self.b.len() {
            Ok(())
        } else {
            Err(err("a frame with bytes left over"))
        }
    }
}

/// Writes one frame.
pub fn write_frame(w: &mut impl Write, payload: &[u8]) -> std::io::Result<()> {
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend((payload.len() as u32).to_le_bytes());
    frame.extend(payload);
    w.write_all(&frame)?;
    w.flush()
}

/// Reads one frame.
pub fn read_frame(r: &mut impl Read) -> std::io::Result<Vec<u8>> {
    let mut n = [0u8; 4];
    r.read_exact(&mut n)?;
    let n = u32::from_le_bytes(n) as usize;
    if n > MAX_FRAME {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("a frame of {n} bytes"),
        ));
    }
    let mut b = vec![0u8; n];
    r.read_exact(&mut b)?;
    Ok(b)
}

/// A model evaluated on an inference server. The model directory is read here as the local backends read it
/// (`Model::load`: the manifest checked against this build, the card rows mapped by name); the server must serve the
/// same model (the same `manifest.json`), which the hello checks.
pub struct RemoteEvaluator {
    address: String,
    model: Model,
    hello: Vec<u8>,
    server: String,
    pool: Mutex<Vec<TcpStream>>,
}

impl RemoteEvaluator {
    /// The model in `dir` on the server at `address` (`HOST:PORT`), its rows mapped from `table`'s. One connection is
    /// made at once, so a server that is not there or refuses the model fails here.
    pub fn connect(
        address: &str,
        dir: &Path,
        table: &CardIndex,
    ) -> Result<RemoteEvaluator, EvalError> {
        let model = Model::load(dir, table)?;
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let hello = hello(model_identity(dir)?, &name);
        let mut e = RemoteEvaluator {
            address: address.to_string(),
            model,
            hello,
            server: String::new(),
            pool: Mutex::new(Vec::new()),
        };
        let (stream, server) = e.open()?;
        e.server = server;
        e.pool
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(stream);
        Ok(e)
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// What the server said of itself when it welcomed the model (its device).
    pub fn server(&self) -> &str {
        &self.server
    }

    fn open(&self) -> Result<(TcpStream, String), EvalError> {
        let mut s = TcpStream::connect(&self.address)
            .map_err(|e| err(format!("the inference server at {}: {e}", self.address)))?;
        s.set_nodelay(true).map_err(err)?;
        s.set_read_timeout(Some(TIMEOUT)).map_err(err)?;
        write_frame(&mut s, &self.hello).map_err(err)?;
        let welcome = read_frame(&mut s)
            .map_err(|e| err(format!("the inference server at {}: {e}", self.address)))?;
        match read_status(&welcome)? {
            Ok(text) => Ok((s, text)),
            Err(why) => Err(err(format!(
                "the inference server at {} refused {}: {why}",
                self.address,
                self.model.dir.display()
            ))),
        }
    }
}

impl Evaluator for RemoteEvaluator {
    fn evaluate(&self, batch: &FeatureBatch) -> Result<Outputs, EvalError> {
        if batch.rows == 0 {
            return Ok(Outputs::default());
        }
        let (payload, order) = request(batch, &self.model.remap)?;
        let pooled = self.pool.lock().unwrap_or_else(|p| p.into_inner()).pop();
        let mut stream = match pooled {
            Some(s) => s,
            None => self.open()?.0,
        };
        write_frame(&mut stream, &payload).map_err(err)?;
        let bytes = read_frame(&mut stream)
            .map_err(|e| err(format!("the inference server at {}: {e}", self.address)))?;
        let out = read_reply(batch, &order, &bytes)?;
        self.pool
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(stream);
        Ok(out)
    }
}
