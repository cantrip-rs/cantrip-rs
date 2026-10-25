//! The AI host (docs/ai-host.md), with the tiny model checked in under `cantrip-ai/tests/data/tiny`:
//!
//! - over stdio, from a scenario at a table: the mirror is made, asked, and answers at once that it started; the
//!   `AiResult` that follows carries an answer the table takes;
//! - a mirror is the table's game: a dealt game whose every answer is the network's, played at a table
//!   (`Session`, as the native host plays it) and mirrored command for command with `Echo`, gives the same reply to
//!   every command, the views and their handles included; and a mirror made late, by `Replay`, is the same game too.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cantrip_ai::TractEvaluator;
use cantrip_ai::record::table;
use cantrip_ai_host::{AiHost, Loaded, Sink};
use cantrip_host::protocol::AnswerPayload;
use cantrip_host::session::{Deal, Session};
use serde_json::{Value, json};

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../cantrip-ai/tests/data/tiny")
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Process {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Process {
    fn start() -> Process {
        let mut child = Command::new(env!("CARGO_BIN_EXE_cantrip-ai-host"))
            .args(["--model", tiny().to_str().unwrap(), "--backend", "tract"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the AI host starts");
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        Process {
            child,
            input,
            output,
        }
    }

    fn send(&mut self, request: Value) {
        let input = self.input.as_mut().unwrap();
        writeln!(input, "{request}").unwrap();
        input.flush().unwrap();
    }

    fn read(&mut self) -> Value {
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("unreadable line {line:?}: {e}"))
    }

    fn call(&mut self, request: Value) -> Value {
        self.send(request);
        self.read()
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        drop(self.input.take());
        if self.child.wait_timeout_or_kill() {
            let _ = self.child.kill();
        }
    }
}

trait WaitOrKill {
    fn wait_timeout_or_kill(&mut self) -> bool;
}

impl WaitOrKill for Child {
    /// Waits a while for the host to exit; whether it had to be killed.
    fn wait_timeout_or_kill(&mut self) -> bool {
        for _ in 0..100 {
            if let Ok(Some(_)) = self.try_wait() {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        true
    }
}

#[test]
fn over_stdio_a_scenario_is_mirrored_asked_and_answered() {
    let name = "lightning-bolt-removes-three-loyalty-from-a-planeswalker";
    let text = std::fs::read_to_string(root().join(format!("scenarios/{name}.cantrip"))).unwrap();
    let mut host = Process::start();
    let models = host.call(json!({ "Cmd": "Models" }));
    assert_eq!(models["Ok"]["Models"], json!(["tiny"]), "{models}");
    assert_eq!(models["Ok"]["Backend"], json!("tract"));

    let made = host.call(json!({ "Cmd": "NewGame", "GameId": 7, "Seats": [], "Scenario": { "Name": name, "Text": text, "Driven": [1] }, "Echo": true }));
    assert_eq!(made["Ok"]["GameId"], json!(7), "{made}");
    let step = host.call(json!({ "Cmd": "Start", "GameId": 7 }));
    let request = &step["Ok"]["Requests"][0];
    assert_eq!(
        request["Seat"],
        json!(1),
        "the driven seat is asked first: {step}"
    );
    let id = request["RequestId"].as_u64().unwrap();

    // Not this seat's request: refused at once.
    let wrong = host.call(json!({ "Cmd": "AiStart", "GameId": 7, "Seat": 2 }));
    assert_eq!(
        wrong["Rejected"]["Code"],
        json!("NotYourTurnToAnswer"),
        "{wrong}"
    );

    let started = host.call(json!({ "Cmd": "AiStart", "GameId": 7, "Seat": 1, "Model": "tiny", "Mode": "Hidden", "Deadline": 5000 }));
    assert_eq!(
        started["Ok"]["Started"]["RequestId"].as_u64(),
        Some(id),
        "{started}"
    );
    assert_eq!(started["Ok"]["Started"]["Mode"], json!("hidden"));
    let result = host.read();
    let r = &result["AiResult"];
    assert_eq!(r["RequestId"].as_u64(), Some(id), "{result}");
    assert_eq!(r["Fallback"], json!(false), "{result}");
    assert_eq!(r["Minted"], json!(false), "{result}");
    assert!(
        r["Steps"].as_u64().unwrap() >= 1 && r["TotalUs"].as_u64().is_some(),
        "{result}"
    );
    let answer: AnswerPayload =
        serde_json::from_value(r["Answer"].clone()).expect("the answer is a client's");

    // The table takes it (whether or not the script meant it).
    let taken = host.call(
        json!({ "Cmd": "Answer", "GameId": 7, "RequestId": id, "Seat": 1, "Answer": answer }),
    );
    assert!(taken.get("Ok").is_some(), "{taken}");

    let model = host.call(json!({ "Cmd": "AiStart", "GameId": 7, "Seat": 1, "Model": "nothing" }));
    assert_eq!(model["Rejected"]["Code"], json!("InvalidSetup"), "{model}");
    let closed = host.call(json!({ "Cmd": "Close", "GameId": 7 }));
    assert!(closed.get("Ok").is_some());
    let gone = host.call(json!({ "Cmd": "Start", "GameId": 7 }));
    assert_eq!(gone["Rejected"]["Code"], json!("TableClosed"));
}

fn deck(name: &str) -> Vec<String> {
    let text = std::fs::read_to_string(root().join("decks").join(name)).unwrap();
    let mut cards = Vec::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let (n, card) = line.split_once(' ').unwrap();
        for _ in 0..n.parse::<usize>().unwrap() {
            cards.push(card.to_string());
        }
    }
    cards
}

/// An AI host in this process, its `AiResult` lines on a channel.
fn in_process() -> (AiHost, Receiver<Value>) {
    let (send, receive) = channel();
    let send = Mutex::new(send);
    let sink: Sink = Arc::new(move |v: &Value| {
        let _ = send.lock().unwrap().send(v.clone());
    });
    let tiny = TractEvaluator::load(&tiny(), table()).unwrap();
    (
        AiHost::new(
            vec![Loaded {
                name: "tiny".into(),
                evaluator: Arc::new(tiny),
            }],
            "tract",
            1,
            sink,
        ),
        receive,
    )
}

fn ok(reply: Value) -> Value {
    match reply {
        Value::Object(mut m) if m.contains_key("Ok") => m.remove("Ok").unwrap(),
        other => panic!("refused: {other}"),
    }
}

#[test]
fn a_mirror_is_the_tables_game_answer_for_answer() {
    let (a, b) = (deck("rw-aggro.txt"), deck("ub-reanimator.txt"));
    let seed = 20261019;
    let mut table = Session::new(Deal {
        seats: vec![
            ("Bot1".into(), a.clone(), Vec::new()),
            ("Bot2".into(), b.clone(), Vec::new()),
        ],
        basic_lands_outside: Vec::new(),
        companions: Vec::new(),
        minimum_deck_size: 0,
        seed,
        shuffle: true,
        first_player: Some(1),
        chooser: None,
    })
    .map_err(|r| r.detail)
    .unwrap();
    let (mut host, results) = in_process();
    let seats = json!([{ "Name": "Bot1", "Deck": a }, { "Name": "Bot2", "Deck": b }]);
    let made = ok(host.line(&json!({ "Cmd": "NewGame", "GameId": 3, "Seats": seats, "Seed": seed, "MinimumDeckSize": 0, "FirstPlayer": 1, "Echo": true }).to_string()));
    assert_eq!(made["GameId"], json!(3));

    let mut commands: Vec<Value> = vec![json!({ "Cmd": "Start" })];
    let mut step = table.step();
    assert_eq!(
        ok(host.line(&json!({ "Cmd": "Start", "GameId": 3 }).to_string())),
        step
    );
    let (mut answered, mut fallbacks) = (0, 0);
    while !step["Finished"].as_bool().unwrap() && answered < 300 {
        let request = &step["Requests"][0];
        let (id, seat) = (
            request["RequestId"].as_u64().unwrap() as u32,
            request["Seat"].as_i64().unwrap() as i32,
        );
        let started =
            ok(host.line(&json!({ "Cmd": "AiStart", "GameId": 3, "Seat": seat }).to_string()));
        assert_eq!(started["Started"]["RequestId"], json!(id));
        let result = results
            .recv_timeout(Duration::from_secs(60))
            .expect("an AiResult");
        let r = &result["AiResult"];
        assert_eq!(r["RequestId"], json!(id), "{result}");
        assert_eq!(
            r["Minted"],
            json!(false),
            "an answer named something the seat has no handle for: {result}"
        );
        if r["Fallback"] == json!(true) {
            fallbacks += 1;
        }
        let answer: AnswerPayload = serde_json::from_value(r["Answer"].clone()).unwrap();
        step = table.answer(id, seat, &answer).unwrap_or_else(|e| {
            panic!(
                "the table refused the network's answer {answer:?}: {} {}",
                e.code, e.detail
            )
        });
        let mirrored = ok(host.line(&json!({ "Cmd": "Answer", "GameId": 3, "RequestId": id, "Seat": seat, "Answer": answer }).to_string()));
        assert_eq!(
            mirrored, step,
            "answer {answered}: the mirror's step is not the table's"
        );
        commands.push(json!({ "Cmd": "Answer", "RequestId": id, "Seat": seat, "Answer": answer }));
        if answered >= 60 {
            let late = ok(host.line(&json!({ "Cmd": "Answer", "GameId": 4, "RequestId": id, "Seat": seat, "Answer": answer }).to_string()));
            assert_eq!(
                late, step,
                "answer {answered}: the replayed mirror's step is not the table's"
            );
        }
        answered += 1;
        // A view mints handles: mirrored too.
        if answered % 7 == 0 {
            let view = json!({ "View": serde_json::to_value(table.view(2).map_err(|r| r.detail).unwrap()).unwrap() });
            assert_eq!(
                ok(host.line(&json!({ "Cmd": "View", "GameId": 3, "Seat": 2 }).to_string())),
                view
            );
            if answered > 60 {
                assert_eq!(
                    ok(host.line(&json!({ "Cmd": "View", "GameId": 4, "Seat": 2 }).to_string())),
                    view
                );
            }
            commands.push(json!({ "Cmd": "View", "Seat": 2 }));
        }
        // Then a mirror made late from the commands so far is the same game from here on.
        if answered == 60 {
            let replayed =
                ok(host.line(&json!({ "Cmd": "NewGame", "GameId": 4, "Seats": seats, "Seed": seed, "MinimumDeckSize": 0, "FirstPlayer": 1, "Echo": true, "Replay": commands }).to_string()));
            assert_eq!(
                replayed["Last"], step,
                "the replayed mirror's last step is not the table's"
            );
        }
    }
    assert!(answered > 60, "the game ended after {answered} answers");
    assert!(
        fallbacks * 4 < answered,
        "{fallbacks} of {answered} answers fell back"
    );
    host.finish();
}

#[test]
fn a_searched_answer_carries_its_search_and_the_table_takes_it() {
    let (a, b) = (deck("rw-aggro.txt"), deck("ub-reanimator.txt"));
    let seed = 20261020;
    let mut table = Session::new(Deal {
        seats: vec![
            ("Bot1".into(), a.clone(), Vec::new()),
            ("Bot2".into(), b.clone(), Vec::new()),
        ],
        basic_lands_outside: Vec::new(),
        companions: Vec::new(),
        minimum_deck_size: 0,
        seed,
        shuffle: true,
        first_player: Some(1),
        chooser: None,
    })
    .map_err(|r| r.detail)
    .unwrap();
    let (mut host, results) = in_process();
    let seats = json!([{ "Name": "Bot1", "Deck": a }, { "Name": "Bot2", "Deck": b }]);
    ok(host.line(&json!({ "Cmd": "NewGame", "GameId": 5, "Seats": seats, "Seed": seed, "MinimumDeckSize": 0, "FirstPlayer": 1, "Echo": true }).to_string()));
    let mut step = table.step();
    assert_eq!(
        ok(host.line(&json!({ "Cmd": "Start", "GameId": 5 }).to_string())),
        step
    );

    // A Hidden search has no prior in this host: refused, before anything is asked.
    let seat = step["Requests"][0]["Seat"].as_i64().unwrap();
    let hidden = host.line(
        &json!({ "Cmd": "AiStart", "GameId": 5, "Seat": seat, "Mode": "Hidden", "Simulations": 8 })
            .to_string(),
    );
    assert_eq!(
        hidden["Rejected"]["Code"],
        json!("InvalidSetup"),
        "{hidden}"
    );

    let (mut answered, mut searched) = (0, 0);
    while !step["Finished"].as_bool().unwrap() && answered < 40 {
        let request = &step["Requests"][0];
        let (id, seat) = (
            request["RequestId"].as_u64().unwrap() as u32,
            request["Seat"].as_i64().unwrap() as i32,
        );
        let start = json!({ "Cmd": "AiStart", "GameId": 5, "Seat": seat, "Mode": "Full", "Simulations": 8, "Worlds": 1, "Batch": 4,
            "Prune": true, "PruneBad": -0.8, "PruneMargin": 0.2, "PruneAfter": 2, "PruneAfterReplies": 1 });
        ok(host.line(&start.to_string()));
        let result = results
            .recv_timeout(Duration::from_secs(60))
            .expect("an AiResult");
        let r = &result["AiResult"];
        assert_eq!(r["RequestId"], json!(id), "{result}");
        if let Some(s) = r.get("Search") {
            searched += 1;
            assert_eq!(s["Simulations"], json!(8), "{result}");
            assert_eq!(s["Worlds"], json!(1), "{result}");
            assert!(
                s["Evaluations"].as_u64().unwrap() >= 1 && s["Nodes"].as_u64().unwrap() >= 1,
                "{result}"
            );
            assert!(
                s["RootValue"].as_f64().unwrap().abs() <= 1.0 + 1e-6,
                "{result}"
            );
            assert!(s["Pruned"].as_u64().is_some(), "{result}");
            assert!(
                s["PrunedImmediate"].as_u64().is_some() && s["PrunedShallow"].as_u64().is_some(),
                "{result}"
            );
        }
        let answer: AnswerPayload = serde_json::from_value(r["Answer"].clone()).unwrap();
        step = table.answer(id, seat, &answer).unwrap_or_else(|e| {
            panic!(
                "the table refused the searched answer {answer:?}: {} {}",
                e.code, e.detail
            )
        });
        assert_eq!(ok(host.line(&json!({ "Cmd": "Answer", "GameId": 5, "RequestId": id, "Seat": seat, "Answer": answer }).to_string())), step);
        answered += 1;
    }
    assert!(
        searched >= 3,
        "{searched} of {answered} answers were searched"
    );
    host.finish();
}
