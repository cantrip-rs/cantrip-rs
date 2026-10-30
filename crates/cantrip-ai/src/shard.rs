//! Training shards (docs/ai-player-plan.md, "Data"): the samples the recorder (`record`) makes of finished games,
//! written as zstd files a trainer reads back (in Rust here, in Python by `ai/shards.py`).
//!
//! A shard is a file of concatenated zstd frames; decompressed, they are one little-endian byte stream:
//!
//! ```text
//! "CTRPSHD1"  u32 n  header: n bytes of JSON (`Header`)
//! then, a frame per game:
//!   u32 n  meta: n bytes of JSON (`GameMeta`)
//!   u32 samples, then each sample:
//!     u8 seat (0-based)  u32 decision  u16 step  u16 turn  i8 outcome  f32 life_swing  u8 dead_next_turn
//!     u16 chosen  u16 v, then v × (u16 option, f32 visits)  u16 h, then h × u32 card row  u32 n, then n bytes of
//!     position (`cantrip_obs::encode::to_compact`)
//! ```
//!
//! A shard is written as `<name>.part` and renamed when it is closed, so a reader never sees half of one; a game is
//! written whole or not at all. Shards roll over at about `max_bytes` compressed or `max_games` games, and are named
//! `<prefix>-<pid>-<start ms>-<n>.shard.zst`, so hosts writing into one directory at once never collide.

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use cantrip_obs::InfoMode;
use cantrip_obs::encode::{SCHEMA_VERSION, encoder_hash};
use serde::{Deserialize, Serialize};

use crate::CardIndex;

pub const MAGIC: &[u8; 8] = b"CTRPSHD1";
/// The layout of the stream above; the observation's own layout is `schema` and `encoder_hash`.
pub const FORMAT: u32 = 1;
/// The engine commit this was built from (`build.rs`).
pub const COMMIT: &str = env!("CANTRIP_COMMIT");
pub const EXTENSION: &str = "shard.zst";
const LEVEL: i32 = 6;

/// What every sample of a shard was made with: a loader refuses a shard whose schema or encoder is not its own, and maps
/// its card rows by `cards`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub format: u32,
    /// `cantrip_obs::encode::SCHEMA_VERSION`.
    pub schema: u32,
    /// `encoder_hash()` as 16 hex digits.
    pub encoder_hash: String,
    /// The card index's hash (`CardIndex::hash`).
    pub text_hash: String,
    /// The card index the samples' card rows are of: `cards[i]` is row `i + 1`. Absent from a shard recorded before
    /// shards named their index; the training side is then told it (docs/ai-training.md).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<String>,
    /// The engine commit (`git rev-parse HEAD` at build time), or "unknown".
    pub commit: String,
    /// What made the answers: `tactical:<model hash>`, or a name.
    pub producer: String,
    /// The seeds the games were dealt from, as the run says them; each game's own is in its meta.
    pub seeds: String,
    /// The information mode of the recorded observations: full, decklist or hidden.
    pub mode: String,
}

impl Header {
    /// The header of shards recorded with `index`.
    pub fn new(index: &CardIndex, producer: &str, seeds: &str, mode: InfoMode) -> Header {
        Header {
            format: FORMAT,
            schema: SCHEMA_VERSION,
            encoder_hash: format!("{:016x}", encoder_hash()),
            text_hash: index.hash.clone(),
            cards: index.names.clone(),
            commit: COMMIT.to_string(),
            producer: producer.to_string(),
            seeds: seeds.to_string(),
            mode: mode_name(mode).to_string(),
        }
    }
}

pub fn mode_name(mode: InfoMode) -> &'static str {
    match mode {
        InfoMode::Full => "full",
        InfoMode::Decklist => "decklist",
        InfoMode::Hidden => "hidden",
    }
}

pub fn parse_mode(text: &str) -> Option<InfoMode> {
    match text.to_ascii_lowercase().as_str() {
        "full" => Some(InfoMode::Full),
        "decklist" => Some(InfoMode::Decklist),
        "hidden" => Some(InfoMode::Hidden),
        _ => None,
    }
}

/// A game's per-decision values as written, a value JSON's `null` read as a NaN: serde_json writes a non-finite f32 so,
/// and a self-play game written before the encoder held its numbers to the f16's range (2026-10-28,
/// `cantrip_obs::encode::f16_bits`) can hold one. The Python reader (`ai/cantrip_ai/dataset.py`) ignores them the same way.
fn values_or_null<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<(u32, f32)>, D::Error> {
    let raw: Vec<(u32, Option<f32>)> = serde::Deserialize::deserialize(d)?;
    Ok(raw.into_iter().map(|(at, v)| (at, v.unwrap_or(f32::NAN))).collect())
}

/// One game: who played it, with what, and how it ended.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GameMeta {
    pub seed: u64,
    /// Each seat's name, seat 1 first.
    pub names: Vec<String>,
    /// Each seat's deck name (the archetype label), empty where the table did not say.
    pub decks: Vec<String>,
    /// Each seat's information mode as recorded.
    pub modes: Vec<String>,
    /// "won", "draw", or "cap" for a game closed unfinished.
    pub ended: String,
    /// The winning seat, from 1.
    pub winner: Option<u8>,
    pub turns: u32,
    /// The answers given along the game's line, every seat's.
    pub answers: u32,
    /// For a searched game (self-play): each searched decision's root value for the answering seat, by the decision's
    /// place (`Sample::decision`). Absent from a game no search played.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "values_or_null")]
    pub root_values: Vec<(u32, f32)>,
    /// Where a table noted one (`Dealt::heuristics`): a heuristic's value of each noted decision's position for the
    /// answering seat, in log-odds of winning (the tactical bot's `TacticalModel`), by the decision's place. Absent from
    /// a game none was noted in: a shard without it reads as before.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "values_or_null")]
    pub heuristic_values: Vec<(u32, f32)>,
    /// The decisions (by place) a fast search of playout-cap randomisation answered: their visits are no policy target,
    /// their positions still value targets (`record::Root::policy`). Absent from a game with none: a shard without it
    /// reads as before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fast_decisions: Vec<u32>,
    /// Where a recorder distilled one (`distill::world_mean`): a perfect-information value's mean over worlds dealt from
    /// the answering seat's knowledge, by the decision's place: a Decklist value's distillation target. Absent from a
    /// game with none.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "values_or_null")]
    pub distill_values: Vec<(u32, f32)>,
}

/// One sub-step a seat answered.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    /// The answering seat, from 0.
    pub seat: u8,
    /// The answer's place among the game's answers.
    pub decision: u32,
    /// The sub-step's place in the answer's decomposition.
    pub step: u16,
    pub turn: u16,
    /// The game's result for `seat`: 1, 0 (a draw or a cap) or -1.
    pub outcome: i8,
    /// The seat's life less its opponents' at the end of the turn, less the same now.
    pub life_swing: f32,
    /// Whether the seat lost by the end of the next turn.
    pub dead_next_turn: bool,
    /// The option chosen, by its place in the step's options.
    pub chosen: u16,
    /// The visit distribution over the options; one-hot on `chosen` for a recorded answer.
    pub visits: Vec<(u16, f32)>,
    /// The opponents' hands, as card rows: the hidden state's labels.
    pub opponent_hand: Vec<u32>,
    /// The encoded position, compact (`cantrip_obs::encode::from_compact`).
    pub position: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameRecord {
    pub meta: GameMeta,
    pub samples: Vec<Sample>,
}

fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn header_bytes(header: &Header) -> Vec<u8> {
    let json = serde_json::to_vec(header).expect("a header serialises");
    let mut out = MAGIC.to_vec();
    put_u32(&mut out, json.len() as u32);
    out.extend_from_slice(&json);
    out
}

/// A game's bytes in the stream.
pub fn game_bytes(game: &GameRecord) -> Vec<u8> {
    let json = serde_json::to_vec(&game.meta).expect("a game's meta serialises");
    let mut out = Vec::with_capacity(
        64 + json.len()
            + game
                .samples
                .iter()
                .map(|s| 40 + s.position.len() + 4 * s.opponent_hand.len())
                .sum::<usize>(),
    );
    put_u32(&mut out, json.len() as u32);
    out.extend_from_slice(&json);
    put_u32(&mut out, game.samples.len() as u32);
    for s in &game.samples {
        out.push(s.seat);
        put_u32(&mut out, s.decision);
        put_u16(&mut out, s.step);
        put_u16(&mut out, s.turn);
        out.push(s.outcome as u8);
        out.extend_from_slice(&s.life_swing.to_le_bytes());
        out.push(s.dead_next_turn as u8);
        put_u16(&mut out, s.chosen);
        put_u16(&mut out, s.visits.len() as u16);
        for &(o, v) in &s.visits {
            put_u16(&mut out, o);
            out.extend_from_slice(&v.to_le_bytes());
        }
        put_u16(&mut out, s.opponent_hand.len() as u16);
        for &r in &s.opponent_hand {
            put_u32(&mut out, r);
        }
        put_u32(&mut out, s.position.len() as u32);
        out.extend_from_slice(&s.position);
    }
    out
}

/// Writes games into shards in a directory, rolling over by size or count.
pub struct ShardWriter {
    dir: PathBuf,
    prefix: String,
    header: Header,
    max_bytes: u64,
    max_games: usize,
    started: u128,
    open: Option<Open>,
    next: usize,
    /// The shards closed so far.
    pub written: Vec<PathBuf>,
    pub games: u64,
    pub samples: u64,
    pub bytes: u64,
}

struct Open {
    file: BufWriter<File>,
    part: PathBuf,
    path: PathBuf,
    bytes: u64,
    games: usize,
}

impl ShardWriter {
    /// Shards of about `max_bytes` compressed or `max_games` games (zero for no limit), into `dir` (made if missing).
    pub fn new(
        dir: &Path,
        prefix: &str,
        header: Header,
        max_bytes: u64,
        max_games: usize,
    ) -> io::Result<ShardWriter> {
        std::fs::create_dir_all(dir)?;
        let started = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        Ok(ShardWriter {
            dir: dir.to_path_buf(),
            prefix: prefix.to_string(),
            header,
            max_bytes,
            max_games,
            started,
            open: None,
            next: 0,
            written: Vec::new(),
            games: 0,
            samples: 0,
            bytes: 0,
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    fn frame(bytes: &[u8]) -> io::Result<Vec<u8>> {
        zstd::bulk::compress(bytes, LEVEL)
    }

    /// Writes a game whole, in a frame of its own.
    pub fn write(&mut self, game: &GameRecord) -> io::Result<()> {
        if self.open.is_none() {
            let name = format!(
                "{}-{}-{}-{:05}",
                self.prefix,
                std::process::id(),
                self.started,
                self.next
            );
            self.next += 1;
            let path = self.dir.join(format!("{name}.{EXTENSION}"));
            let part = self.dir.join(format!("{name}.{EXTENSION}.part"));
            let mut file = BufWriter::new(File::create(&part)?);
            let head = Self::frame(&header_bytes(&self.header))?;
            file.write_all(&head)?;
            self.open = Some(Open {
                file,
                part,
                path,
                bytes: head.len() as u64,
                games: 0,
            });
        }
        let frame = Self::frame(&game_bytes(game))?;
        let open = self.open.as_mut().expect("opened above");
        open.file.write_all(&frame)?;
        open.bytes += frame.len() as u64;
        open.games += 1;
        self.games += 1;
        self.samples += game.samples.len() as u64;
        self.bytes += frame.len() as u64;
        if (self.max_bytes > 0 && open.bytes >= self.max_bytes)
            || (self.max_games > 0 && open.games >= self.max_games)
        {
            self.close()?;
        }
        Ok(())
    }

    /// Closes the shard being written, if any: it is renamed to its final name.
    pub fn close(&mut self) -> io::Result<()> {
        if let Some(mut open) = self.open.take() {
            open.file.flush()?;
            open.file.get_ref().sync_all()?;
            drop(open.file);
            std::fs::rename(&open.part, &open.path)?;
            self.written.push(open.path);
        }
        Ok(())
    }
}

impl Drop for ShardWriter {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn bad(what: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.into())
}

/// Reads a shard back: its header, then its games one at a time.
pub struct ShardReader<R: Read> {
    stream: R,
    pub header: Header,
}

impl ShardReader<zstd::stream::read::Decoder<'static, BufReader<File>>> {
    pub fn open(path: &Path) -> io::Result<Self> {
        ShardReader::new(zstd::stream::read::Decoder::new(File::open(path)?)?)
    }
}

fn take<const N: usize>(r: &mut impl Read) -> io::Result<[u8; N]> {
    let mut b = [0u8; N];
    r.read_exact(&mut b)?;
    Ok(b)
}

fn u16_(r: &mut impl Read) -> io::Result<u16> {
    take::<2>(r).map(u16::from_le_bytes)
}

fn u32_(r: &mut impl Read) -> io::Result<u32> {
    take::<4>(r).map(u32::from_le_bytes)
}

fn f32_(r: &mut impl Read) -> io::Result<f32> {
    take::<4>(r).map(f32::from_le_bytes)
}

fn bytes(r: &mut impl Read, n: usize) -> io::Result<Vec<u8>> {
    let mut b = vec![0u8; n];
    r.read_exact(&mut b)?;
    Ok(b)
}

impl<R: Read> ShardReader<R> {
    /// A reader of the decompressed stream.
    pub fn new(mut stream: R) -> io::Result<Self> {
        if &take::<8>(&mut stream)? != MAGIC {
            return Err(bad("not a cantrip shard"));
        }
        let n = u32_(&mut stream)? as usize;
        let header = serde_json::from_slice(&bytes(&mut stream, n)?)
            .map_err(|e| bad(format!("the header: {e}")))?;
        Ok(ShardReader { stream, header })
    }

    /// The next game, `None` at the end of the shard.
    pub fn game(&mut self) -> io::Result<Option<GameRecord>> {
        let mut first = [0u8; 4];
        match self.stream.read(&mut first[..1])? {
            0 => return Ok(None),
            _ => self.stream.read_exact(&mut first[1..])?,
        }
        let r = &mut self.stream;
        let n = u32::from_le_bytes(first) as usize;
        let meta: GameMeta = serde_json::from_slice(&bytes(r, n)?)
            .map_err(|e| bad(format!("a game's meta: {e}")))?;
        let count = u32_(r)? as usize;
        let mut samples = Vec::with_capacity(count);
        for _ in 0..count {
            let [seat] = take::<1>(r)?;
            let decision = u32_(r)?;
            let step = u16_(r)?;
            let turn = u16_(r)?;
            let [outcome] = take::<1>(r)?;
            let life_swing = f32_(r)?;
            let [dead] = take::<1>(r)?;
            let chosen = u16_(r)?;
            let v = u16_(r)? as usize;
            let mut visits = Vec::with_capacity(v);
            for _ in 0..v {
                visits.push((u16_(r)?, f32_(r)?));
            }
            let h = u16_(r)? as usize;
            let mut opponent_hand = Vec::with_capacity(h);
            for _ in 0..h {
                opponent_hand.push(u32_(r)?);
            }
            let n = u32_(r)? as usize;
            let position = bytes(r, n)?;
            samples.push(Sample {
                seat,
                decision,
                step,
                turn,
                outcome: outcome as i8,
                life_swing,
                dead_next_turn: dead != 0,
                chosen,
                visits,
                opponent_hand,
                position,
            });
        }
        Ok(Some(GameRecord { meta, samples }))
    }
}

/// A shard whole: its header and every game.
pub fn read(path: &Path) -> io::Result<(Header, Vec<GameRecord>)> {
    let mut reader = ShardReader::open(path)?;
    let mut games = Vec::new();
    while let Some(g) = reader.game()? {
        games.push(g);
    }
    Ok((reader.header, games))
}

/// The finished shards in a directory, sorted.
pub fn shards(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut all: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(&format!(".{EXTENSION}")))
        })
        .collect();
    all.sort();
    Ok(all)
}
