//! The shard stream's formats (`cantrip_ai::shard`): format 2 carries each sample's `searched` flag and its visits'
//! root values (`q`) and reads back as written; a format 1 shard, written before them, still reads, its samples not
//! searched and with no q.

use std::io::Read;

use cantrip_ai::record::table;
use cantrip_ai::shard::{self, FORMAT, GameMeta, GameRecord, Header, Sample, ShardReader, ShardWriter};
use cantrip_obs::InfoMode;

fn sample(searched: bool, q: Vec<f32>) -> Sample {
    Sample {
        seat: 1,
        decision: 7,
        step: 0,
        turn: 3,
        outcome: -1,
        life_swing: 2.5,
        dead_next_turn: false,
        chosen: 1,
        visits: if searched { vec![(0, 0.25), (1, 0.75)] } else { vec![(1, 1.0)] },
        searched,
        q,
        opponent_hand: vec![4, 9],
        position: vec![1, 2, 3, 4, 5],
    }
}

fn game() -> GameRecord {
    GameRecord {
        meta: GameMeta { seed: 5, ended: "won".into(), winner: Some(1), turns: 3, answers: 9, ..GameMeta::default() },
        samples: vec![sample(true, vec![-0.25, 0.5]), sample(false, Vec::new())],
    }
}

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cantrip-shard-format-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn format_2_carries_searched_and_q() {
    assert_eq!(FORMAT, 2);
    let dir = temp("two");
    let header = Header::new(table(), "test", "0..1", InfoMode::Full);
    let mut w = ShardWriter::new(&dir, "t", header, 0, 0).unwrap();
    w.write(&game()).unwrap();
    w.close().unwrap();
    let (header, games) = shard::read(&w.written[0]).unwrap();
    assert_eq!(header.format, 2);
    assert_eq!(games, vec![game()]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A format 1 stream, as the recorder wrote it before: the same game less the flags byte and the q values.
#[test]
fn a_format_1_shard_still_reads() {
    let mut header = Header::new(table(), "old", "0..1", InfoMode::Full);
    header.format = 1;
    let json = serde_json::to_vec(&header).unwrap();
    let mut out = shard::MAGIC.to_vec();
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    let g = game();
    let meta = serde_json::to_vec(&g.meta).unwrap();
    out.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    out.extend_from_slice(&meta);
    out.extend_from_slice(&(g.samples.len() as u32).to_le_bytes());
    for s in &g.samples {
        out.push(s.seat);
        out.extend_from_slice(&s.decision.to_le_bytes());
        out.extend_from_slice(&s.step.to_le_bytes());
        out.extend_from_slice(&s.turn.to_le_bytes());
        out.push(s.outcome as u8);
        out.extend_from_slice(&s.life_swing.to_le_bytes());
        out.push(s.dead_next_turn as u8);
        out.extend_from_slice(&s.chosen.to_le_bytes());
        out.extend_from_slice(&(s.visits.len() as u16).to_le_bytes());
        for &(o, v) in &s.visits {
            out.extend_from_slice(&o.to_le_bytes());
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&(s.opponent_hand.len() as u16).to_le_bytes());
        for &r in &s.opponent_hand {
            out.extend_from_slice(&r.to_le_bytes());
        }
        out.extend_from_slice(&(s.position.len() as u32).to_le_bytes());
        out.extend_from_slice(&s.position);
    }
    let mut reader = ShardReader::new(std::io::Cursor::new(out)).unwrap();
    assert_eq!(reader.header.format, 1);
    let read = reader.game().unwrap().unwrap();
    assert!(reader.game().unwrap().is_none());
    let want: Vec<Sample> = g.samples.iter().map(|s| Sample { searched: false, q: Vec::new(), ..s.clone() }).collect();
    assert_eq!(read.samples, want);
}

/// A format the reader does not know is refused, not misread.
#[test]
fn an_unknown_format_is_refused() {
    let mut header = Header::new(table(), "new", "0..1", InfoMode::Full);
    header.format = 99;
    let json = serde_json::to_vec(&header).unwrap();
    let mut out = shard::MAGIC.to_vec();
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    let err = ShardReader::new(std::io::Cursor::new(out).take(1 << 20)).err().expect("refused");
    assert!(err.to_string().contains("format 99"), "{err}");
}
