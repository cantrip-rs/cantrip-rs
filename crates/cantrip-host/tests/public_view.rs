//! The public view (docs/host-protocol.md, "The public view"): the game as a viewer who is no player sees it, for an
//! in-person table's shared boards. It holds no hand, no sideboard and no library top; a face-down permanent or spell stays
//! face down to it while its controller knows it; a revealed card is shown and a looked-at one is not; it waits while any
//! seat is part way through a cast; taking back a mana ability restores its handles and retracts what it was told. Each
//! step's map from its handles to each seat's pairs an object only while it lives and the seat names it by that handle.
//!
//! Every position is forced: a scenario played at a table (its Bot1 driven, every answer the one its script means), or one
//! whole game dealt from the benchmark's fixed decks, checked at every step.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cantrip_host::protocol::*;
use cantrip_host::session::{Deal, Session};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A step's public part, read back: its view, its events, and by seat its handle map (public handle to the seat's).
struct Public {
    view: PlayerView,
    events: Vec<Value>,
    maps: HashMap<i32, HashMap<u32, u32>>,
}

fn public_of(step: &Value) -> Public {
    let public = &step["Public"];
    assert!(public.is_object(), "a game dealt with a public view has it on every step: {step}");
    let view: PlayerView = serde_json::from_value(public["View"].clone()).expect("the public view reads back");
    let events = public["Events"].as_array().expect("public events").clone();
    let mut maps = HashMap::new();
    for (seat, pairs) in public["Handles"].as_object().expect("the handle maps") {
        let mut map = HashMap::new();
        for pair in pairs.as_array().unwrap() {
            let (p, s) = (pair[0].as_u64().unwrap() as u32, pair[1].as_u64().unwrap() as u32);
            assert!(map.insert(p, s).is_none(), "public handle {p} is paired twice for seat {seat}");
        }
        maps.insert(seat.parse().unwrap(), map);
    }
    Public { view, events, maps }
}

fn seat_views(step: &Value) -> HashMap<i32, PlayerView> {
    step["Views"].as_object().unwrap().iter().map(|(seat, v)| (seat.parse().unwrap(), serde_json::from_value(v.clone()).unwrap())).collect()
}

fn of_type<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
    events.iter().filter(|e| e["$type"] == kind).collect()
}

/// What every step's public part must be, whatever the position. `last` is the public view of the step before, `revealed`
/// every public handle a reveal named so far.
fn assert_sound(step: &Value, last: &Option<PlayerView>, revealed: &mut HashSet<u32>) {
    let public = public_of(step);
    let seats = seat_views(step);
    assert_eq!(public.view.viewer, 0, "the public view is no seat's");
    for e in of_type(&public.events, "CardsRevealedEvent") {
        revealed.extend(e["Cards"].as_array().unwrap().iter().map(|h| h.as_u64().unwrap() as u32));
    }
    // A look is that player's alone: the public hears that it happened, never of what - unless the card is revealed too.
    for e in of_type(&public.events, "CardsLookedAtEvent") {
        for h in e["Cards"].as_array().unwrap().iter().map(|h| h.as_u64().unwrap() as u32) {
            assert!(h == 0 || revealed.contains(&h), "the public is told what was looked at: {e}");
        }
    }
    for zone in &public.view.zones {
        match zone.kind {
            Some(ZoneKindView::Hand | ZoneKindView::Library) => {
                assert!(zone.contents.is_empty() && !zone.contents_visible, "the public looks into {:?}: {zone:?}", zone.kind)
            }
            Some(ZoneKindView::Sideboard) => assert!(!zone.contents_visible, "the public looks into a sideboard"),
            _ => {}
        }
    }
    for o in &public.view.objects {
        // A hidden zone's card is shown to the public only as a reveal shows it to everyone.
        if matches!(o.zone, Some(ZoneKindView::Hand | ZoneKindView::Library)) {
            assert!(revealed.contains(&o.handle.0), "the public sees {} in a {:?} it was never shown", o.name, o.zone);
        }
        if o.face_down && !o.known {
            assert!(o.name.is_empty() && o.face_down_name.is_empty() && o.card_id.is_empty(), "a face-down object is named to the public: {o:?}");
        }
    }
    // The pairs name one object, alike to both.
    for (seat, map) in &public.maps {
        let view = &seats[seat];
        for (p, s) in map {
            if let (Some(po), Some(so)) = (public.view.objects.iter().find(|o| o.handle.0 == *p), view.objects.iter().find(|o| o.handle.0 == *s)) {
                assert_eq!((po.zone, po.owner, po.controller), (so.zone, so.owner, so.controller), "seat {seat} pairs {p} with {s}");
                if !po.face_down {
                    assert_eq!(po.name, so.name, "seat {seat} pairs {p} with {s}");
                }
            }
        }
    }
    // A view worked out afresh is worked out with no seat waiting on a run (`deferred`): everything it shows, each seat sees
    // too, at least as well.
    if last.as_ref().is_some_and(|l| serde_json::to_value(l).unwrap() == serde_json::to_value(&public.view).unwrap()) {
        return;
    }
    for o in &public.view.objects {
        for (seat, view) in &seats {
            let Some(theirs) = public.maps[seat].get(&o.handle.0) else {
                panic!("the public sees {} ({:?}, handle {}) and seat {seat} does not name it", o.name, o.zone, o.handle.0)
            };
            let theirs = view.objects.iter().find(|x| x.handle.0 == *theirs).unwrap_or_else(|| panic!("seat {seat} has no object {theirs}"));
            if o.known {
                assert!(theirs.known, "the public knows {} and seat {seat} does not", o.face_down_name);
            }
        }
    }
}

/// Plays the scenario `name` at a table with a public view, Bot1 driven, checking every step (`assert_sound`) and handing it
/// to `look` with the seats' views and the public part. Returns the last step.
fn play(name: &str, mut look: impl FnMut(&Value, &Public, &HashMap<i32, PlayerView>)) -> Value {
    let text = std::fs::read_to_string(root().join(format!("scenarios/{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    session.show_public();
    let mut step = session.step();
    let mut last = None;
    let mut revealed = HashSet::new();
    for _ in 0..20_000 {
        assert_sound(&step, &last, &mut revealed);
        let public = public_of(&step);
        look(&step, &public, &seat_views(&step));
        last = Some(public.view);
        let scenario = &step["Scenario"];
        if scenario["Verdict"].is_object() {
            assert_eq!(scenario["Verdict"]["Passed"], Value::Bool(true), "{name}: {}", scenario["Verdict"]["Report"]);
            return step;
        }
        let expected = &scenario["Expected"];
        let answer: AnswerPayload = if expected["Undo"] == Value::Bool(true) {
            AnswerPayload::TakeActionAnswer(TakeActionAnswer::new(ActionKindView::UndoManaAbility, ViewHandle::NONE))
        } else {
            serde_json::from_value(expected["Answer"].clone()).unwrap()
        };
        let id = expected["RequestId"].as_u64().unwrap() as u32;
        let seat = expected["Seat"].as_i64().unwrap() as i32;
        step = session.answer(id, seat, &answer).map_err(|r| r.detail).unwrap();
    }
    panic!("{name} went on too long at the table");
}

fn prompt_kind(step: &Value) -> Option<String> {
    step["Requests"][0]["Prompt"]["$type"].as_str().map(str::to_string)
}

/// A game dealt without a public view says nothing of one.
#[test]
fn a_game_dealt_without_a_public_view_has_none() {
    let text = std::fs::read_to_string(root().join("scenarios/opt-scries-one-and-draws-the-card-left-on-top.cantrip")).unwrap();
    let mut session = Session::scenario(&text, "opt", Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    assert!(session.step().get("Public").is_none());
}

/// A face-down spell and permanent (morph, CR 708.4, 708.5): a nameless 2/2 to the public while its controller knows it,
/// and what it is once turned face up.
#[test]
fn a_morph_is_face_down_to_the_public_while_its_controller_knows_it() {
    let (mut on_stack, mut on_battlefield, mut face_up) = (false, false, false);
    play("abzan-guide-cast-face-down-for-three-turned-face-up-for-exactly-its-morph-cost", |_, public, seats| {
        for o in public.view.objects.iter().filter(|o| o.face_down) {
            assert!(!o.known && o.name.is_empty() && o.face_down_name.is_empty(), "{o:?}");
            let mine = seats[&1].objects.iter().find(|x| Some(&x.handle.0) == public.maps[&1].get(&o.handle.0)).expect("Bot1 sees its morph");
            assert!(mine.known && mine.face_down_name == "Abzan Guide", "{mine:?}");
            match o.zone {
                Some(ZoneKindView::Stack) => on_stack = true,
                Some(ZoneKindView::Battlefield) => on_battlefield = true,
                _ => {}
            }
        }
        face_up |= public.view.objects.iter().any(|o| o.name == "Abzan Guide" && !o.face_down && o.zone == Some(ZoneKindView::Battlefield));
    });
    assert!(on_stack && on_battlefield && face_up, "{on_stack} {on_battlefield} {face_up}");
}

/// A foretold card is exiled face down from its owner's hand (CR 702.143a, 406.3): its owner may look at it, Bot2 and the
/// public may not.
#[test]
fn a_foretold_card_is_face_down_to_the_public() {
    let mut seen = false;
    play("augury-raven-foretell-paid-exactly-and-cast-from-exile-for-the-foretell-cost", |_, public, seats| {
        for o in public.view.objects.iter().filter(|o| o.face_down && o.zone == Some(ZoneKindView::Exile)) {
            seen = true;
            assert!(!o.known && o.name.is_empty() && o.face_down_name.is_empty(), "{o:?}");
            let mine = seats[&1].objects.iter().find(|x| Some(&x.handle.0) == public.maps[&1].get(&o.handle.0)).expect("Bot1 sees it");
            assert!(mine.known, "{mine:?}");
        }
    });
    assert!(seen, "the foretold card was in exile");
}

/// A manifested card (CR 701.40a) is a face-down 2/2 to the public while its controller knows it.
#[test]
fn a_manifested_card_is_face_down_to_the_public() {
    let mut seen = false;
    play("abhorrent-oculus-manifests-dread-on-an-opponents-upkeep", |_, public, _| {
        for o in public.view.objects.iter().filter(|o| o.face_down && o.zone == Some(ZoneKindView::Battlefield)) {
            assert!(!o.known && o.name.is_empty(), "{o:?}");
            seen = true;
        }
    });
    assert!(seen, "the manifested card was on the battlefield");
}

/// Bolas's Citadel lets Bot1 look at the top of its library any time (CR 401.2): the card is in Bot1's view, and the public
/// has no handle for it.
#[test]
fn the_top_of_a_library_its_player_may_look_at_is_not_the_publics() {
    let mut seen = 0;
    play("bolass-citadel-plays-off-the-top-for-life", |_, public, seats| {
        for top in seats[&1].objects.iter().filter(|o| o.zone == Some(ZoneKindView::Library)) {
            seen += 1;
            assert!(!public.maps[&1].values().any(|&h| h == top.handle.0), "the public names Bot1's library top {}", top.name);
            assert!(!public.view.objects.iter().any(|o| o.name == top.name && o.zone == Some(ZoneKindView::Library)), "{}", top.name);
        }
    });
    assert!(seen > 0, "Bot1 looked at its library's top");
}

/// Duress makes Bot2 reveal its hand (CR 701.20a): the public is shown the cards that step, named.
#[test]
fn a_revealed_hand_is_shown_to_the_public() {
    let mut shown = Vec::new();
    play("duress-takes-the-named-card", |_, public, _| {
        for e in of_type(&public.events, "CardsRevealedEvent") {
            for h in e["Cards"].as_array().unwrap() {
                let o = public.view.objects.iter().find(|o| h.as_u64() == Some(u64::from(o.handle.0))).expect("a revealed card is in the public view");
                shown.push(o.name.clone());
            }
        }
    });
    shown.sort();
    assert_eq!(shown, ["Counterspell", "Guerrilla Gorilla", "Island", "Island", "Island", "Island", "Lightning Bolt"]);
}

/// Gitaxian Probe looks at Bot2's hand (CR 701.20e): Bot1 is told what it looked at, the public only that it
/// looked, and it names none of those cards.
#[test]
fn a_looked_at_card_is_not_shown_to_the_public() {
    let mut looked = false;
    play("gitaxian-probe-looks-at-the-target-players-hand", |step, public, _| {
        let mine = of_type(step["Events"]["1"].as_array().unwrap(), "CardsLookedAtEvent");
        if !mine.is_empty() {
            looked = true;
            assert_eq!(of_type(&public.events, "CardsLookedAtEvent").len(), mine.len(), "the public hears of the look");
            for h in mine.iter().flat_map(|e| e["Cards"].as_array().unwrap()) {
                let h = h.as_u64().unwrap() as u32;
                assert!(h != 0 && !public.maps[&1].values().any(|&v| v == h), "the public names a card Bot1 looked at");
            }
        }
    });
    assert!(looked);
}

/// Delver of Secrets looks at the top of its library and may reveal it (CR 701.20a): a card revealed is shown to the public,
/// one kept hidden never is. In the first game Bot1 reveals an Island and keeps a Lightning Bolt hidden; in the second it
/// reveals the Bolt.
#[test]
fn a_card_looked_at_is_shown_to_the_public_only_if_revealed() {
    for (name, expected) in [
        ("delver-of-secrets-reveals-a-land-or-keeps-an-instant-hidden-and-stays-delver", ["Island"]),
        ("delver-of-secrets-reveals-an-instant-and-transforms-into-insectile-aberration", ["Lightning Bolt"]),
    ] {
        let mut shown = Vec::new();
        play(name, |_, public, _| {
            for o in public.view.objects.iter().filter(|o| o.zone == Some(ZoneKindView::Library)) {
                shown.push(o.name.clone());
            }
        });
        assert_eq!(shown, expected, "{name}");
    }
}

/// While a seat is part way through a cast (announcing, paying), the public keeps the view it was last sent, as the other
/// seat does: no spell being announced, no land tapped to pay for it.
#[test]
fn the_public_view_waits_while_a_seat_is_casting() {
    let (mut held, mut before, mut theirs) = (0, None::<Value>, Value::Null);
    play("a-land-tapped-while-paying-is-taken-back-and-the-cast-once-paid-is-not", |step, public, _| {
        let now = serde_json::to_value(&public.view).unwrap();
        if prompt_kind(step).as_deref() == Some("PayCostsPromptMessage") {
            held += 1;
            assert_eq!(Some(&now), before.as_ref(), "the public view moved while Bot1 paid");
            assert!(public.events.is_empty(), "the public is told of a cast being paid for: {:?}", public.events);
            assert_eq!(step["Views"]["2"], theirs, "Bot2 waits too");
        } else {
            before = Some(now);
            theirs = step["Views"]["2"].clone();
        }
    });
    assert!(held > 0, "Bot1 paid for Pentad Prism");
}

/// Taking back a Treasure sacrificed for mana (`cantrip_engine::undo`): the public is told the retraction of exactly what
/// Bot2 is, and the Treasure is on the battlefield again under the public handle it had before.
#[test]
fn taking_back_a_mana_ability_restores_the_publics_handles() {
    let mut before: Option<u32> = None;
    let mut gone = false;
    let mut restored = false;
    play("a-treasure-sacrificed-for-mana-is-taken-back", |step, public, _| {
        let treasure = public.view.objects.iter().find(|o| o.name == "Treasure Token" && o.controller == 1 && o.zone == Some(ZoneKindView::Battlefield));
        let retracted = of_type(&public.events, "EventsRetractedEvent");
        if !retracted.is_empty() {
            let theirs = of_type(step["Events"]["2"].as_array().unwrap(), "EventsRetractedEvent");
            assert_eq!(retracted.iter().map(|e| e["Sequences"].clone()).collect::<Vec<_>>(), theirs.iter().map(|e| e["Sequences"].clone()).collect::<Vec<_>>());
            let treasure = treasure.expect("the Treasure is back");
            assert_eq!(Some(treasure.handle.0), before, "under the handle it had");
            assert!(public.maps[&1].contains_key(&treasure.handle.0) && public.maps[&2].contains_key(&treasure.handle.0));
            restored = true;
            return;
        }
        match treasure {
            Some(t) if !restored => before = Some(t.handle.0),
            None if before.is_some() && !restored => {
                // Sacrificed: a token that left the battlefield ceased to exist (CR 111.8), so no seat's handle is paired with
                // the one the public had for it.
                gone = true;
                let h = before.unwrap();
                assert!(!public.maps[&1].contains_key(&h) && !public.maps[&2].contains_key(&h), "a dead object is paired");
            }
            _ => {}
        }
    });
    assert!(gone && restored, "{gone} {restored}");
}

/// Emrakul gives Bot1 control of Bot2 for a turn (CR 723.4): Bot1 sees Bot2's hand then, and the public still does not.
#[test]
fn a_controlled_players_hand_is_not_the_publics() {
    let mut controlled = false;
    play("emrakul-the-promised-end-controls-the-next-turn-then-gives-an-extra-one-and-is-protected-from-instants", |_, public, seats| {
        let theirs = seats[&1].zones.iter().find(|z| z.kind == Some(ZoneKindView::Hand) && z.owner == 2).unwrap();
        if theirs.contents_visible && !theirs.contents.is_empty() {
            controlled = true;
            for h in &theirs.contents {
                assert!(!public.maps[&1].values().any(|v| v == &h.0), "the public names a card of Bot2's hand");
            }
        }
    });
    assert!(controlled, "Bot1 saw Bot2's hand");
}

fn load_deck(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut cards = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let (n, name) = line.split_once(' ').unwrap();
        cards.extend(std::iter::repeat_n(name.trim().to_string(), n.parse().unwrap()));
    }
    cards
}

/// Plays a whole game from the prompts alone, as the bot of `tests/stdio_game.rs` does, the decks the benchmark's (Duress,
/// Thought Scour, Phantasmal Image among them), dealt the one fixed way: every step's public part is sound (`assert_sound`),
/// and the public view is worked out to the game's end.
#[test]
fn a_whole_game_projects_soundly_for_the_public() {
    let deal = Deal {
        seats: vec![
            ("North".into(), load_deck(&root().join("decks/spike/spike-rg.txt")), Vec::new()),
            ("South".into(), load_deck(&root().join("decks/spike/spike-ubw.txt")), Vec::new()),
        ],
        basic_lands_outside: Vec::new(),
        companions: Vec::new(),
        minimum_deck_size: 40,
        seed: 3,
        shuffle: true,
        first_player: Some(1),
        chooser: None,
    };
    let mut session = Session::new(deal).map_err(|r| r.detail).unwrap();
    session.show_public();
    let mut step = session.step();
    let (mut last, mut revealed) = (None, HashSet::new());
    let mut allow_cast = true;
    let mut steps = 0;
    while !step["Finished"].as_bool().unwrap() && steps < 20_000 {
        steps += 1;
        assert_sound(&step, &last, &mut revealed);
        last = Some(public_of(&step).view);
        if step["Events"].as_object().unwrap().values().any(|list| !of_type(list.as_array().unwrap(), "StepBeganEvent").is_empty()) {
            allow_cast = true;
        }
        let request = &step["Requests"][0];
        let prompt: PromptPayload = serde_json::from_value(request["Prompt"].clone()).unwrap();
        let seat = request["Seat"].as_i64().unwrap() as i32;
        let id = request["RequestId"].as_u64().unwrap() as u32;
        step = match session.answer(id, seat, &choose(&prompt, &mut allow_cast)) {
            Ok(step) => step,
            Err(_) => session.answer(id, seat, &prompt.default_answer()).map_err(|r| r.detail).expect("the default answer is taken"),
        };
    }
    assert_sound(&step, &last, &mut revealed);
    assert!(step["Finished"].as_bool().unwrap(), "the game ended within {steps} steps");
    assert_eq!(public_of(&step).view.outcome, serde_json::from_value(step["Outcome"].clone()).unwrap());
}

fn choose(prompt: &PromptPayload, allow_cast: &mut bool) -> AnswerPayload {
    match prompt {
        PromptPayload::PriorityPromptMessage(p) => {
            let pick = |kind: ActionKindView, quiet: bool| p.options.iter().find(|o| o.kind == kind && (!quiet || o.produces.is_empty()));
            match pick(ActionKindView::PlayLand, false).or(pick(ActionKindView::CastSpell, false).filter(|_| *allow_cast)).or(pick(ActionKindView::ActivateAbility, true)) {
                Some(o) => AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o)),
                None => AnswerPayload::PassAnswer(PassAnswer {}),
            }
        }
        PromptPayload::PayCostsPromptMessage(p) => {
            let pick = |kind: ActionKindView| p.options.iter().find(|o| o.kind == kind);
            if let Some(o) = pick(ActionKindView::ConfirmCasting) {
                return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
            }
            if let Some(o) = p.options.iter().find(|o| o.kind == ActionKindView::ActivateAbility && !o.produces.is_empty()) {
                let mut answer = TakeActionAnswer::take(o);
                answer.color = o.color_choices.first().copied();
                return AnswerPayload::TakeActionAnswer(answer);
            }
            if let Some(o) = pick(ActionKindView::AssignMana) {
                *allow_cast = false;
                return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
            }
            *allow_cast = false;
            prompt.default_answer()
        }
        other => other.default_answer(),
    }
}
