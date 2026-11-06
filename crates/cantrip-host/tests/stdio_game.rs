//! Drives the host binary through whole games over stdio, the way `PhaseRulesHost` does: JSON lines in, one reply per
//! line out. A small bot plays both seats from the prompts alone (handles, never object ids), so this also proves
//! every prompt it meets names only what its seat can see and that every answer it gives is taken.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use cantrip_host::protocol::*;
use serde_json::{Value, json};

struct Host {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Host {
    fn start() -> Host {
        let mut child = Command::new(env!("CARGO_BIN_EXE_cantrip-native-host"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the host starts");
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Host { child, input, output }
    }

    fn call(&mut self, request: Value) -> Value {
        writeln!(self.input, "{request}").unwrap();
        self.input.flush().unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("unreadable reply {line:?}: {e}"))
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
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

/// Plays land, casts what it can, taps lands for mana, attacks with everything; anything else takes the default.
fn choose(prompt: &PromptPayload) -> AnswerPayload {
    match prompt {
        PromptPayload::PriorityPromptMessage(p) => {
            if let Some(o) = p.options.iter().find(|o| o.kind == ActionKindView::PlayLand) {
                return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
            }
            for o in p.options.iter().filter(|o| o.kind == ActionKindView::CastSpell && o.ability_index == 0 && !o.has_x) {
                if o.targets_required > 0 && o.target_candidates.len() < o.targets_required as usize {
                    continue;
                }
                let mut answer = TakeActionAnswer::take(o);
                answer.targets = o.target_candidates.iter().take(o.targets_required as usize).cloned().collect();
                return AnswerPayload::TakeActionAnswer(answer);
            }
            AnswerPayload::PassAnswer(PassAnswer {})
        }
        PromptPayload::PayCostsPromptMessage(p) => {
            let pick = |kind: ActionKindView| p.options.iter().find(|o| o.kind == kind);
            if let Some(o) = p.options.iter().find(|o| o.kind == ActionKindView::ActivateAbility && o.source != p.spell && (!o.produces.is_empty() || o.produces_of_any_one_color > 0)) {
                let mut answer = TakeActionAnswer::take(o);
                answer.color = o.color_choices.first().copied();
                return AnswerPayload::TakeActionAnswer(answer);
            }
            if let Some(o) = pick(ActionKindView::ConfirmCasting) {
                return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
            }
            if let Some(o) = pick(ActionKindView::AssignMana) {
                return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
            }
            prompt.default_answer()
        }
        PromptPayload::DeclareAttackersPromptMessage(p) => AnswerPayload::AttackersDeclaredAnswer(AttackersDeclaredAnswer {
            attacks: p
                .eligible
                .iter()
                .filter_map(|&a| p.attacks_for(a).into_iter().next())
                .collect(),
        }),
        other => other.default_answer(),
    }
}

fn expect_ok(reply: &Value) -> &Value {
    reply.get("Ok").unwrap_or_else(|| panic!("the host refused: {reply}"))
}

#[test]
fn a_whole_game_over_stdio() {
    let mut host = Host::start();
    let dealt = host.call(json!({
        "Cmd": "NewGame",
        "Seats": [ { "Name": "North", "Deck": deck() }, { "Name": "South", "Deck": deck() } ],
        "Seed": 7, "Shuffle": true, "FirstPlayer": 1, "MinimumDeckSize": 40,
    }));
    let game = expect_ok(&dealt)["GameId"].as_u64().unwrap();
    assert_eq!(expect_ok(&dealt)["Decklists"]["1"].as_array().unwrap().len(), 40);

    let mut reply = host.call(json!({ "Cmd": "Start", "GameId": game }));
    let mut answered = 0;
    let mut cast = 0;
    let mut refused = 0;
    let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
    loop {
        let step = expect_ok(&reply).clone();
        for list in step["Events"].as_object().unwrap().values() {
            for event in list.as_array().unwrap() {
                *kinds.entry(event["$type"].as_str().unwrap().to_string()).or_default() += 1;
            }
        }
        if step["Finished"].as_bool().unwrap() {
            assert!(matches!(step["Outcome"].as_str(), Some("TeamWon" | "Draw")), "{step}");
            break;
        }
        let requests = step["Requests"].as_array().unwrap();
        assert_eq!(requests.len(), 1, "one question is outstanding: {step}");
        let request = &requests[0];
        let seat = request["Seat"].as_i64().unwrap();
        let prompt: PromptPayload = serde_json::from_value(request["Prompt"].clone()).unwrap();
        assert_eq!(i64::from(prompt.player()), seat);
        // Each seat sees its own hand and not the other's.
        let views = &step["Views"];
        let mine: PlayerView = serde_json::from_value(views[seat.to_string()].clone()).unwrap();
        let theirs: PlayerView = serde_json::from_value(views[(3 - seat).to_string()].clone()).unwrap();
        let hand_of = |view: &PlayerView, owner: i64| view.zones.iter().find(|z| z.kind == Some(ZoneKindView::Hand) && i64::from(z.owner) == owner).unwrap().clone();
        assert!(hand_of(&mine, seat).contents_visible);
        assert!(!hand_of(&theirs, seat).contents_visible && hand_of(&theirs, seat).contents.is_empty());
        assert!(theirs.objects.iter().all(|o| !(i64::from(o.owner) == seat && o.zone == Some(ZoneKindView::Hand))));
        if matches!(prompt, PromptPayload::PayCostsPromptMessage(_)) {
            cast += 1;
        }
        let answer = choose(&prompt);
        reply = host.call(json!({ "Cmd": "Answer", "GameId": game, "RequestId": request["RequestId"], "Seat": seat, "Answer": answer }));
        if reply.get("Rejected").is_some() {
            refused += 1;
            // Not everything the bot tries is legal; the game is as it was and the default is.
            reply = host.call(json!({ "Cmd": "Answer", "GameId": game, "RequestId": request["RequestId"], "Seat": seat, "Answer": prompt.default_answer() }));
        }
        answered += 1;
        assert!(answered < 20_000, "the game does not end");
    }
    assert!(cast > 0, "something was cast");
    eprintln!("answered {answered}, refused {refused}, events {kinds:?}");
    for kind in ["StepBeganEvent", "ObjectMovedEvent", "LifeChangedEvent", "SpellCastEvent", "GameEndedEvent", "PlayerLostEvent"] {
        assert!(kinds.contains_key(kind), "no {kind} was told: {kinds:?}");
    }
    assert!(refused * 20 < answered, "{refused} of {answered} answers were refused");

    let closed = host.call(json!({ "Cmd": "Close", "GameId": game }));
    assert!(closed.get("Ok").is_some());
}

#[test]
fn stale_and_foreign_answers_are_refused() {
    let mut host = Host::start();
    let dealt = host.call(json!({
        "Cmd": "NewGame",
        "Seats": [ { "Name": "North", "Deck": deck() }, { "Name": "South", "Deck": deck() } ],
        "Seed": 1, "Shuffle": true, "FirstPlayer": 1, "MinimumDeckSize": 40,
    }));
    let game = expect_ok(&dealt)["GameId"].as_u64().unwrap();
    let step = host.call(json!({ "Cmd": "Start", "GameId": game }));
    let request = expect_ok(&step)["Requests"][0].clone();
    let id = request["RequestId"].as_u64().unwrap();
    let seat = request["Seat"].as_i64().unwrap();
    let pass = json!({ "$type": "PassAnswer" });

    let stale = host.call(json!({ "Cmd": "Answer", "GameId": game, "RequestId": id + 99, "Seat": seat, "Answer": pass }));
    assert_eq!(stale["Rejected"]["Code"], "StaleReply");
    let foreign = host.call(json!({ "Cmd": "Answer", "GameId": game, "RequestId": id, "Seat": 3 - seat, "Answer": pass }));
    assert_eq!(foreign["Rejected"]["Code"], "NotYourTurnToAnswer");
    let unknown = host.call(json!({ "Cmd": "View", "GameId": 99, "Seat": 1 }));
    assert_eq!(unknown["Rejected"]["Code"], "TableClosed");
    let oracle = host.call(json!({ "Cmd": "Oracle", "Name": "Lightning Bolt" }));
    assert_eq!(expect_ok(&oracle)["Card"]["Name"], "Lightning Bolt");
    let conceded = host.call(json!({ "Cmd": "Concede", "GameId": game, "Seat": seat }));
    assert_eq!(expect_ok(&conceded)["Finished"], true);
}
