//! `--ai-record DIR`: the host records the games that end as training shards, and recording changes nothing a table
//! sees. The same games are played over stdio by the same bot with and without the flag, and every reply must be the
//! same; then the host's stdin is closed, and the shards it leaves hold the games that ended (a game played out, one
//! conceded, one closed early as a cap) and not the one left unfinished.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use cantrip_ai::shard;
use cantrip_host::protocol::*;
use serde_json::{Value, json};

struct Host {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Host {
    fn start(args: &[&str]) -> Host {
        let mut child = Command::new(env!("CARGO_BIN_EXE_cantrip-native-host"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the host starts");
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        Host {
            child,
            input,
            output,
        }
    }

    fn call(&mut self, request: Value) -> Value {
        let input = self.input.as_mut().unwrap();
        writeln!(input, "{request}").unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("unreadable reply {line:?}: {e}"))
    }

    /// Closes stdin and waits for the host to finish (and so to close its shards).
    fn finish(mut self) {
        drop(self.input.take());
        let status = self.child.wait().unwrap();
        assert!(status.success(), "the host exited {status}");
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if self.input.is_some() {
            let _ = self.child.kill();
        }
    }
}

fn deck() -> Vec<String> {
    let mut deck = Vec::new();
    for _ in 0..12 {
        deck.push("Mountain".to_string());
        deck.push("Forest".to_string());
    }
    for _ in 0..6 {
        deck.push("Lightning Bolt".to_string());
        deck.push("Llanowar Elves".to_string());
    }
    for _ in 0..4 {
        deck.push("Goblin Engineer".to_string());
    }
    deck
}

/// Plays a land, casts what it can, taps for mana, attacks with everything; anything else takes the default.
fn choose(prompt: &PromptPayload) -> AnswerPayload {
    match prompt {
        PromptPayload::PriorityPromptMessage(p) => {
            if let Some(o) = p
                .options
                .iter()
                .find(|o| o.kind == ActionKindView::PlayLand)
            {
                return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
            }
            for o in p
                .options
                .iter()
                .filter(|o| o.kind == ActionKindView::CastSpell && o.ability_index == 0 && !o.has_x)
            {
                if o.targets_required > 0 && o.target_candidates.len() < o.targets_required as usize
                {
                    continue;
                }
                let mut answer = TakeActionAnswer::take(o);
                answer.targets = o
                    .target_candidates
                    .iter()
                    .take(o.targets_required as usize)
                    .cloned()
                    .collect();
                return AnswerPayload::TakeActionAnswer(answer);
            }
            AnswerPayload::PassAnswer(PassAnswer {})
        }
        PromptPayload::PayCostsPromptMessage(p) => {
            if let Some(o) = p
                .options
                .iter()
                .find(|o| o.kind == ActionKindView::ConfirmCasting)
            {
                return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
            }
            if let Some(o) = p
                .options
                .iter()
                .find(|o| o.kind == ActionKindView::ActivateAbility && !o.produces.is_empty())
            {
                let mut answer = TakeActionAnswer::take(o);
                answer.color = o.color_choices.first().copied();
                return AnswerPayload::TakeActionAnswer(answer);
            }
            if let Some(o) = p
                .options
                .iter()
                .find(|o| o.kind == ActionKindView::AssignMana)
            {
                return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
            }
            prompt.default_answer()
        }
        PromptPayload::DeclareAttackersPromptMessage(p) => {
            AnswerPayload::AttackersDeclaredAnswer(AttackersDeclaredAnswer {
                attacks: p
                    .eligible
                    .iter()
                    .filter_map(|&a| p.attacks_for(a).into_iter().next())
                    .collect(),
            })
        }
        other => other.default_answer(),
    }
}

/// Plays a game for at most `answers` answers (or to its end), recording every reply.
fn play(host: &mut Host, seed: u64, answers: usize, transcript: &mut Vec<Value>) -> (u64, bool) {
    let dealt = host.call(json!({
        "Cmd": "NewGame",
        "Seats": [ { "Name": "North", "Deck": deck(), "DeckName": "rg-north" }, { "Name": "South", "Deck": deck(), "DeckName": "rg-south" } ],
        "Seed": seed, "Shuffle": true, "FirstPlayer": 1, "MinimumDeckSize": 0,
    }));
    let game = dealt["Ok"]["GameId"].as_u64().unwrap();
    transcript.push(dealt);
    let mut reply = host.call(json!({ "Cmd": "Start", "GameId": game }));
    for _ in 0..answers {
        transcript.push(reply.clone());
        let step = reply["Ok"].clone();
        if step["Finished"].as_bool().unwrap() {
            return (game, true);
        }
        let request = &step["Requests"][0];
        let seat = request["Seat"].as_i64().unwrap();
        let prompt: PromptPayload = serde_json::from_value(request["Prompt"].clone()).unwrap();
        reply = host.call(json!({ "Cmd": "Answer", "GameId": game, "RequestId": request["RequestId"], "Seat": seat, "Answer": choose(&prompt) }));
        if reply.get("Rejected").is_some() {
            transcript.push(reply);
            reply = host.call(json!({ "Cmd": "Answer", "GameId": game, "RequestId": request["RequestId"], "Seat": seat, "Answer": prompt.default_answer() }));
        }
    }
    transcript.push(reply);
    (game, false)
}

/// Four games: one played out, one conceded, one closed early, one left unfinished.
fn session(host: &mut Host) -> Vec<Value> {
    let mut t = Vec::new();
    let (_, finished) = play(host, 7, 20_000, &mut t);
    assert!(finished, "the first game ends");
    let (conceded, _) = play(host, 8, 60, &mut t);
    t.push(host.call(json!({ "Cmd": "Concede", "GameId": conceded, "Seat": 2 })));
    let (capped, _) = play(host, 9, 40, &mut t);
    t.push(host.call(json!({ "Cmd": "Close", "GameId": capped })));
    play(host, 10, 30, &mut t);
    t
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cantrip-host-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn recording_changes_no_reply_and_writes_the_games_that_ended() {
    let mut plain = Host::start(&[]);
    let without = session(&mut plain);
    plain.finish();

    let dir = temp("ai-record");
    let path = dir.to_string_lossy().to_string();
    let mut recording = Host::start(&[
        "--ai-record",
        &path,
        "--ai-record-producer",
        "test",
        "--ai-record-seeds",
        "7..10",
        "--ai-record-threads",
        "2",
    ]);
    let with = session(&mut recording);
    recording.finish();
    assert_eq!(without.len(), with.len());
    for (k, (a, b)) in without.iter().zip(&with).enumerate() {
        assert_eq!(a, b, "reply {k} differs with recording on");
    }

    let shards = shard::shards(&dir).unwrap();
    assert!(!shards.is_empty(), "no shards in {}", dir.display());
    let mut games = Vec::new();
    for s in &shards {
        let (header, g) = shard::read(s).unwrap();
        assert_eq!(
            (
                header.producer.as_str(),
                header.seeds.as_str(),
                header.mode.as_str()
            ),
            ("test", "7..10", "full")
        );
        games.extend(g);
    }
    games.sort_by_key(|g| g.meta.seed);
    let seeds: Vec<u64> = games.iter().map(|g| g.meta.seed).collect();
    assert_eq!(seeds, vec![7, 8, 9], "the unfinished game is dropped");
    assert_eq!(
        games[0].meta.decks,
        vec!["rg-north".to_string(), "rg-south".to_string()]
    );
    assert_eq!(games[0].meta.ended, "won");
    assert_eq!(
        (games[1].meta.ended.as_str(), games[1].meta.winner),
        ("won", Some(1))
    );
    assert_eq!(games[2].meta.ended, "cap");
    assert!(games[2].samples.iter().all(|s| s.outcome == 0));
    for g in &games {
        assert!(!g.samples.is_empty(), "game {} has no samples", g.meta.seed);
        let winner = g.meta.winner.map(|w| w - 1);
        for s in &g.samples {
            let want = match winner {
                Some(w) if w == s.seat => 1,
                Some(_) => -1,
                None => 0,
            };
            assert_eq!(s.outcome, want);
        }
    }
    assert!(reports(&dir) == 1, "one report");
    let _ = std::fs::remove_dir_all(&dir);
}

fn reports(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("report-"))
        .count()
}
