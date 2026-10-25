//! Every scenario of `scenarios/*.cantrip` played at a table (docs/scenarios-in-the-browser.md): Bot1's seat driven, its
//! every answer the one the host's step says the script means, sent back over the wire as a client would send it. A
//! scenario that passes alone must pass so: what the host says to answer is what the script means, in the seat's own
//! handles.

use std::path::{Path, PathBuf};

use cantrip_host::protocol::{ActionKindView, AnswerPayload, TakeActionAnswer, ViewHandle};
use cantrip_host::session::Session;
use cantrip_script::{GameScript, known_bug, run};
use serde_json::Value;

fn scripts() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scenarios")
}

/// Plays one scenario at a table with Bot1 driven. Ok, or the verdict's report.
fn at_a_table(name: &str, text: &str) -> Result<(), String> {
    let mut session = Session::scenario(text, name, Vec::new(), &[1]).map_err(|r| r.detail)?;
    let mut step = session.step();
    for _ in 0..20_000 {
        let scenario = &step["Scenario"];
        if let Some(verdict) = scenario["Verdict"].as_object() {
            return if verdict["Passed"] == Value::Bool(true) { Ok(()) } else { Err(verdict["Report"].as_str().unwrap_or("").to_string()) };
        }
        let expected = &scenario["Expected"];
        if expected.is_null() {
            return Err(format!("the step says nothing to answer and gives no verdict: {}", step["Requests"]));
        }
        let answer: AnswerPayload = if expected["Undo"] == Value::Bool(true) {
            AnswerPayload::TakeActionAnswer(TakeActionAnswer::new(ActionKindView::UndoManaAbility, ViewHandle::NONE))
        } else {
            serde_json::from_value(expected["Answer"].clone()).map_err(|e| format!("the expected answer does not read back: {e}"))?
        };
        let id = expected["RequestId"].as_u64().unwrap() as u32;
        let seat = expected["Seat"].as_i64().unwrap() as i32;
        step = session.answer(id, seat, &answer).map_err(|r| format!("{}: {} ({:?}, line {})", r.code, r.detail, answer, expected["Line"]))?;
    }
    Err("the scenario went on too long at the table".into())
}

/// Every string a step's requests hold under the key `key`, for every option of the card `source` (a handle) when given.
fn fields(value: &Value, key: &str, source: Option<&Value>, out: &mut Vec<(String, Value)>) {
    match value {
        Value::Object(fields_of) => {
            if fields_of.get("Kind").and_then(Value::as_str) == Some("CastSpell")
                && source.is_none_or(|s| fields_of.get("Source") == Some(s))
                && let Some(v) = fields_of.get(key)
            {
                let description = fields_of.get("Description").and_then(Value::as_str).unwrap_or("").to_owned();
                out.push((description, v.clone()));
            }
            for v in fields_of.values() {
                fields(v, key, source, out);
            }
        }
        Value::Array(values) => values.iter().for_each(|v| fields(v, key, source, out)),
        _ => {}
    }
}

/// Web-slinging returns a tapped creature, chosen once the mana is paid (CR 702.188a, 601.2g-h): its cast row says so, the
/// card's ordinary cast row does not.
#[test]
fn a_web_slinging_cast_row_says_its_creature_is_chosen_after_the_mana() {
    let (mut saw_web, mut saw_printed) = (false, false);
    play_looking("spider-sense-web-slinging-returns-the-birds-tapped-for-its-own-mana", |_, views, step| {
        if let Some(card) = views[0].objects.iter().find(|o| o.name == "Spider-Sense") {
            let mut rows = Vec::new();
            fields(&step["Requests"], "TappedAfterMana", Some(&serde_json::to_value(card.handle).unwrap()), &mut rows);
            for (description, tapped) in rows {
                let web = description.contains("Web-slinging");
                assert_eq!(tapped, Value::Bool(web), "{description}");
                if web { saw_web = true } else { saw_printed = true }
            }
        }
        false
    });
    assert!(saw_web && saw_printed);
}

/// Two permissions that would each let a card be cast for the same alternative cost (Lurrus of the Dream-Den's and Serra
/// Paragon's, each with Primal Prayers' {E}) are two rows, each naming the permanent whose permission it is (CR 601.2b).
#[test]
fn rows_alike_in_cost_name_the_permission_each_is_cast_by() {
    let mut saw_both = false;
    play_looking("primal-prayers-chooses-between-granted-graveyard-permissions", |_, _, step| {
        let mut rows = Vec::new();
        fields(&step["Requests"], "Description", None, &mut rows);
        let has = |name: &str| rows.iter().any(|(d, _)| d.contains("Pay {E}") && d.contains(&format!("({name})")));
        saw_both |= has("Lurrus of the Dream-Den") && has("Serra Paragon");
        false
    });
    assert!(saw_both, "no prompt showed both energy casts with their permissions");
}

/// Hidden information (CR 406.3, 708.2): Headliner Scarlett exiles Bot2's top card, Firdoch Core, face down. What Bot1's
/// view says of that card - at the table, in the views as they are sent - carries nothing of what it is: no name, and not
/// that it is every creature type (Changeling), which would say it.
#[test]
fn a_face_down_changeling_in_exile_tells_the_opponent_nothing_of_what_it_is() {
    let name = "headliner-scarlett-face-down-firdoch-core-shows-an-opponent-no-types";
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    let mut step = session.step();
    let mut seen = 0;
    for _ in 0..20_000 {
        let view = session.view(1).map_err(|r| r.detail).unwrap();
        for object in view.objects.iter().filter(|o| o.face_down) {
            seen += 1;
            assert!(!object.every_creature_type, "a face-down card says it is every creature type: {:?}", object.handle);
            assert!(object.type_line_subtypes.is_empty() && object.subtypes.is_empty(), "a face-down card names types");
            assert!(object.granted_abilities.is_empty() && object.lost_abilities.is_empty() && object.name.is_empty());
        }
        let scenario = &step["Scenario"];
        if scenario["Verdict"].is_object() {
            break;
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
    assert!(seen > 0, "the scenario never showed Bot1 a face-down card");
}

/// A restriction on casting is shown to the player it binds (CR 101.2): while Avatar's Wrath bars Bot2 from casting outside
/// its hand, Bot2's view says so beside its life total and Bot1's says it of Bot2, and once Bot1's next turn has begun
/// neither says it any more.
#[test]
fn a_bar_on_casting_outside_the_hand_is_shown_to_both_players_while_it_lasts() {
    let name = "avatars-wrath-bars-its-opponent-casting-from-exile-and-graveyard-until-its-casters-next-turn";
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    let mut step = session.step();
    let (mut shown, mut gone_after) = (0, false);
    for _ in 0..20_000 {
        let own = session.view(2).map_err(|r| r.detail).unwrap();
        let other = session.view(1).map_err(|r| r.detail).unwrap();
        let facts = |view: &cantrip_host::protocol::PlayerView| view.players[1].pending_effects.clone();
        let (own_facts, other_facts) = (facts(&own), facts(&other));
        let bot1 = own.players[0].name.clone();
        let barred = own_facts.contains(&format!("You can't cast spells from anywhere other than your hand until {bot1}'s next turn."));
        assert_eq!(
            barred,
            other_facts.contains(&"They can't cast spells from anywhere other than their hand until your next turn.".to_string()),
            "the two players are told different things: {own_facts:?} / {other_facts:?}"
        );
        if barred {
            shown += 1;
            assert!(!gone_after, "the restriction came back");
        } else if shown > 0 {
            gone_after = true;
        }
        let scenario = &step["Scenario"];
        if scenario["Verdict"].is_object() {
            assert_eq!(scenario["Verdict"]["Passed"], Value::Bool(true));
            break;
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
    assert!(shown > 0, "Bot2 was never told it can't cast outside its hand");
    assert!(gone_after, "the restriction was still shown after Bot1's next turn began");
}

/// "Players can't gain life" is shown to both players while it applies (CR 119.7): while Bot1's Sunspine Lynx is on the
/// battlefield, each player's view says it of them ("You can't gain life.") and of the other ("They can't gain life."), and once
/// Swords to Plowshares has exiled the Lynx neither says it any more.
#[test]
fn players_cant_gain_life_is_shown_to_both_players_while_the_static_ability_applies() {
    let name = "sunspine-lynx-stops-life-gain-only-while-it-is-on-the-battlefield";
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    let mut step = session.step();
    let (mut shown, mut gone_after) = (0, false);
    for _ in 0..20_000 {
        let views = [session.view(1).map_err(|r| r.detail).unwrap(), session.view(2).map_err(|r| r.detail).unwrap()];
        let says = |viewer: usize, about: usize, text: &str| views[viewer].players[about].pending_effects.iter().any(|f| f == text);
        let applies = says(0, 0, "You can't gain life.");
        for (viewer, about) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
            let text = if viewer == about { "You can't gain life." } else { "They can't gain life." };
            assert_eq!(says(viewer, about, text), applies, "seat {} is told something else of seat {}", viewer + 1, about + 1);
        }
        if applies {
            shown += 1;
            assert!(!gone_after, "the restriction came back");
        } else if shown > 0 {
            gone_after = true;
        }
        let scenario = &step["Scenario"];
        if scenario["Verdict"].is_object() {
            assert_eq!(scenario["Verdict"]["Passed"], Value::Bool(true));
            break;
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
    assert!(shown > 0, "no player was ever told they can't gain life");
    assert!(gone_after, "the restriction was still shown after the Lynx left");
}

/// Plays the scenario `name` at a table and judges what each seat is told, beside Bot1's life total, of a delayed triggered
/// ability that waits for its event (CR 603.7b) whose words contain `words`: nothing before it is set up, then Bot1's own view
/// names its source and gives the words, Bot2's says it is Bot1's ("They have ..."), and once it has lapsed neither does.
fn judge_waiting_delayed_ability(name: &str, words: &str, source: &str) {
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    let mut step = session.step();
    let (mut shown, mut gone_after, mut first) = (0, false, true);
    for _ in 0..20_000 {
        let mut facts = |seat: i32| session.view(seat).map_err(|r| r.detail).unwrap().players[0].pending_effects.clone();
        let (own, other) = (facts(1), facts(2));
        let own_line = own.iter().find(|f| f.contains(words));
        let other_line = other.iter().find(|f| f.contains(words));
        assert_eq!(own_line.is_some(), other_line.is_some(), "the two seats are told different things: {own:?} / {other:?}");
        if first {
            assert!(own_line.is_none(), "the ability was shown before anything set it up: {own:?}");
            first = false;
        }
        if let (Some(mine), Some(theirs)) = (own_line, other_line) {
            assert!(mine.starts_with(&format!("{source}: ")), "Bot1 is not told where it came from: {mine}");
            assert!(theirs.starts_with(&format!("They have {source}'s delayed ability: ")), "Bot2 is told something else: {theirs}");
            shown += 1;
            assert!(!gone_after, "the waiting ability came back");
        } else if shown > 0 {
            gone_after = true;
        }
        let scenario = &step["Scenario"];
        if scenario["Verdict"].is_object() {
            assert_eq!(scenario["Verdict"]["Passed"], Value::Bool(true));
            break;
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
    assert!(shown > 0, "the waiting ability was never shown");
    assert!(gone_after, "the waiting ability was still shown after it lapsed");
}

/// "Whenever you attack this turn" (Dalkovan Encampment's activated ability) is shown from the moment it is set up until the
/// turn ends.
#[test]
fn a_this_turn_delayed_ability_is_shown_until_the_turn_ends() {
    judge_waiting_delayed_ability(
        "dalkovan-encampment-shows-the-waiting-attack-ability-and-the-warriors-in-the-browser",
        "Whenever you attack this turn",
        "Dalkovan Encampment",
    );
}

/// An "until your next turn" delayed ability (Tamiyo, Seasoned Scholar's +2) is shown until that turn begins.
#[test]
fn an_until_your_next_turn_delayed_ability_is_shown_until_that_turn_begins() {
    judge_waiting_delayed_ability(
        "tamiyo-seasoned-scholar-plus-two-weakens-creatures-attacking-her-too",
        "Whenever a creature attacks you or a planeswalker you control",
        "Tamiyo, Seasoned Scholar",
    );
}

/// CR 305.7 at a table: under Magus of the Moon, Bot2's Azorius Chancery is a Mountain. Each seat's view of it, at the end of
/// the scenario, gives its subtypes as Mountain alone and calls the three lines its rules text printed lost: "This land enters
/// tapped." (a rule of its card, not an ability among its abilities), its enters trigger, and its "{T}: Add {W}{U}." Its
/// name stays.
#[test]
fn a_nonbasic_land_magus_of_the_moon_makes_a_mountain_is_shown_with_its_printed_lines_lost() {
    let name = "magus-of-the-moon-lets-azorius-chancery-enter-untapped-without-its-trigger";
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    let mut step = session.step();
    for _ in 0..20_000 {
        let scenario = &step["Scenario"];
        if scenario["Verdict"].is_object() {
            assert_eq!(scenario["Verdict"]["Passed"], Value::Bool(true));
            break;
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
    for seat in [1, 2] {
        let view = session.view(seat).map_err(|r| r.detail).unwrap();
        let chancery = view.objects.iter().find(|o| o.name == "Azorius Chancery").expect("the Chancery is shown");
        assert_eq!(chancery.subtypes, vec!["Mountain".to_string()], "seat {seat}");
        for line in ["This land enters tapped.", "When this land enters, return a land you control to its owner's hand.", "{T}: Add {W}{U}."] {
            assert!(chancery.lost_abilities.iter().any(|l| l == line), "seat {seat} is not told {line:?} is lost: {:?}", chancery.lost_abilities);
        }
        assert!(chancery.granted_abilities.is_empty(), "seat {seat}: {:?}", chancery.granted_abilities.iter().map(|a| &a.text).collect::<Vec<_>>());
    }
}

/// CR 605.1a, 305.6 at a table: under Blood Sun ("All lands lose all abilities except mana abilities") Taiga keeps the mana
/// abilities of its land types, so the reminder text that is its whole text box is not shown as lost.
#[test]
fn a_land_under_blood_sun_is_not_shown_losing_its_land_types_mana_abilities() {
    let name = "blood-sun-leaves-taiga-the-mana-abilities-of-its-land-types";
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    let mut step = session.step();
    for _ in 0..20_000 {
        let scenario = &step["Scenario"];
        if scenario["Verdict"].is_object() {
            assert_eq!(scenario["Verdict"]["Passed"], Value::Bool(true));
            break;
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
    for seat in [1, 2] {
        let view = session.view(seat).map_err(|r| r.detail).unwrap();
        let taiga = view.objects.iter().find(|o| o.name == "Taiga").expect("Taiga is shown");
        assert!(taiga.lost_abilities.is_empty(), "seat {seat}: {:?}", taiga.lost_abilities);
    }
}

#[test]
fn every_scenario_passes_at_a_table_with_bot1_driven() {
    let mut entries: Vec<PathBuf> =
        std::fs::read_dir(scripts()).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "cantrip")).collect();
    entries.sort();
    // Each scenario: None when it does not run alone, else whether it passed at the table.
    let outcomes: Vec<Option<Result<(), String>>> = on_every_core(&entries, |path| {
        let text = std::fs::read_to_string(path).unwrap();
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let script = GameScript::parse(&text, &name).ok()?;
        if known_bug(&text).is_some() || script.cards().into_iter().any(|c| cantrip_engine::card(&c).is_none()) {
            return None;
        }
        run(&script).ok()?;
        Some(at_a_table(&name, &text).map_err(|report| format!("{name}: {}", report.lines().take(3).collect::<Vec<_>>().join(" | "))))
    });
    let played = outcomes.iter().flatten().count();
    let failed: Vec<String> = outcomes.into_iter().flatten().filter_map(Result::err).collect();
    assert!(failed.is_empty(), "{} of {played} scenarios fail at a table:\n{}", failed.len(), failed.join("\n"));
}

/// `each` of `paths`, run on every core: a loop over every scenario on one thread takes minutes. The results in the
/// order of `paths`.
fn on_every_core<T: Send>(paths: &[PathBuf], each: impl Fn(&Path) -> T + Sync) -> Vec<T> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut done: Vec<(usize, T)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = paths.get(i) else { return mine };
                        mine.push((i, each(path)));
                    }
                })
            })
            .collect();
        workers.into_iter().flat_map(|w| w.join().unwrap()).collect()
    });
    done.sort_by_key(|&(i, _)| i);
    done.into_iter().map(|(_, t)| t).collect()
}

/// A spell being announced and paid for (CR 601.2a-h) is the caster's alone until it is cast (CR 601.2i), as the events
/// already have it: Bot2's view, as each step sends it, shows no Lightning Bolt on the stack and no Mountain tapped while
/// Bot1 is choosing its target and paying - a cast that is taken back is never seen - and shows the Bolt once it is cast.
#[test]
fn a_spell_being_announced_is_not_shown_to_the_opponent_until_it_is_cast() {
    let text = "[Bot1 Deck]\nMountain\nLightning Bolt\nMountain x18\n\n[Bot2 Deck]\nIsland x20\n\n[Actions]\nBot1.Hand.Mountain.Play\nBot1.Hand.Lightning Bolt.Cast(Bot2)\nBot1.Battlefield.Mountain.Activate\n\n[Expect]\nBot2.Life = 17\n";
    let mut session = Session::scenario(text, "announced-bolt", Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    let mut step = session.step();
    let stack = |view: &Value| -> i64 {
        view["Zones"].as_array().unwrap().iter().find(|z| z["Kind"] == "Stack").map_or(0, |z| z["Count"].as_i64().unwrap())
    };
    let tapped = |view: &Value| view["Objects"].as_array().unwrap().iter().filter(|o| o["Tapped"] == Value::Bool(true)).count();
    let (mut announcing, mut cast) = (0, 0);
    for _ in 0..20_000 {
        let (own, other) = (&step["Views"]["1"], &step["Views"]["2"]);
        if stack(own) > 0 && stack(other) == 0 {
            announcing += 1;
            assert_eq!(tapped(other), 0, "Bot2 saw a Mountain tapped to pay for a spell still being cast");
            assert_eq!(other["Players"][0]["HandCount"], 6, "Bot2 saw the Bolt leave Bot1's hand");
        }
        if stack(other) > 0 {
            cast += 1;
        }
        let scenario = &step["Scenario"];
        if scenario["Verdict"].is_object() {
            assert_eq!(scenario["Verdict"]["Passed"], Value::Bool(true), "{}", scenario["Verdict"]["Report"]);
            break;
        }
        let expected = &scenario["Expected"];
        let answer: AnswerPayload = serde_json::from_value(expected["Answer"].clone()).unwrap();
        let id = expected["RequestId"].as_u64().unwrap() as u32;
        let seat = expected["Seat"].as_i64().unwrap() as i32;
        step = session.answer(id, seat, &answer).map_err(|r| r.detail).unwrap();
    }
    assert!(announcing > 0, "Bot1 was never asked anything while the Bolt was being announced");
    assert!(cast > 0, "Bot2 never saw the Bolt on the stack once it was cast");
}

/// Outrageous Robbery (CR 406.3): "You may look at" the cards it exiles face down. Bot1, who looked, is told what the card is
/// (`face_down_name`, `known`); Bot2, whose library it came out of and who never looked at it, is told nothing of it - no
/// name, not known - though it is a card of Bot2's in Bot2's own exile.
#[test]
fn a_card_robbery_exiles_face_down_is_named_to_the_caster_and_not_to_its_owner() {
    let name = "outrageous-robbery-in-the-browser-offers-the-caster-the-face-down-card-in-the-opponents-exile";
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    let mut step = session.step();
    let (mut caster_saw, mut owner_saw) = (0, 0);
    for _ in 0..20_000 {
        let caster = session.view(1).map_err(|r| r.detail).unwrap();
        let owner = session.view(2).map_err(|r| r.detail).unwrap();
        for object in caster.objects.iter().filter(|o| o.face_down) {
            assert_eq!(object.face_down_name, "Lightning Helix", "the caster is not told what it looked at");
            assert!(object.known && object.name.is_empty());
            caster_saw += 1;
        }
        for object in owner.objects.iter().filter(|o| o.face_down) {
            assert!(object.face_down_name.is_empty() && !object.known && object.name.is_empty(), "the owner is told what the card is: {:?}", object.face_down_name);
            owner_saw += 1;
        }
        let scenario = &step["Scenario"];
        if scenario["Verdict"].is_object() {
            assert_eq!(scenario["Verdict"]["Passed"], Value::Bool(true));
            break;
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
    assert!(caster_saw > 0 && owner_saw > 0, "the scenario never showed both seats the face-down card");
}

/// Plays the scenario `name` at a table, Bot1 driven by the script, calling `look` with both seats' views and the step before
/// each answer; stops early when `look` says so. The last step.
fn play_looking(name: &str, mut look: impl FnMut(&mut Session, &[cantrip_host::protocol::PlayerView; 2], &Value) -> bool) -> Value {
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).map_err(|r| r.detail).unwrap();
    let mut step = session.step();
    for _ in 0..20_000 {
        let views = [session.view(1).map_err(|r| r.detail).unwrap(), session.view(2).map_err(|r| r.detail).unwrap()];
        if look(&mut session, &views, &step) {
            return step;
        }
        let scenario = &step["Scenario"];
        if scenario["Verdict"].is_object() {
            assert_eq!(scenario["Verdict"]["Passed"], Value::Bool(true), "{}", scenario["Verdict"]["Report"]);
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
    panic!("the scenario went on too long at the table");
}

/// Whether `viewer`'s view (0 or 1) says `text` beside the life total of `about`.
fn says(views: &[cantrip_host::protocol::PlayerView; 2], viewer: usize, about: usize, text: &str) -> bool {
    views[viewer].players[about].pending_effects.iter().any(|f| f == text)
}

/// A game drawn by a loop of mandatory actions (CR 104.4b) says so to every seat: the three-Oblivion-Ring loop ends with a
/// `GameEndedEvent` told to both players, a draw, and its reason the loop - what the client's game-over panel and log say.
#[test]
fn a_game_drawn_by_a_mandatory_loop_tells_every_seat_why() {
    let mut ended: Vec<(String, Value)> = Vec::new();
    let last = play_looking("three-oblivion-rings-with-no-other-nonland-permanent-loop-forever-and-the-game-is-a-draw", |_, _, step| {
        for (seat, list) in step["Events"].as_object().unwrap() {
            for event in list.as_array().unwrap().iter().filter(|e| e["$type"] == "GameEndedEvent") {
                ended.push((seat.clone(), event.clone()));
            }
        }
        false
    });
    assert_eq!(last["Outcome"], "Draw");
    let mut seats: Vec<&str> = ended.iter().map(|(s, _)| s.as_str()).collect();
    seats.sort_unstable();
    assert_eq!(seats, ["1", "2"], "each seat is told once: {ended:?}");
    for (_, event) in &ended {
        assert_eq!(event["Outcome"], "Draw");
        assert_eq!(event["DrawReason"], "MandatoryLoop", "{event}");
    }
}

/// Platinum Angel's state is shown to both players (CR 104.2b, 104.3): while it is out, Bot1 is told "You can't lose the game."
/// and Bot2 that Bot1 can't ("They can't lose the game."), and Bot2 that it can't win, which Bot1 is told of Bot2. At 0 life
/// a concession still loses (CR 104.3a): the game is over, Bot2 the winner.
#[test]
fn platinum_angel_is_shown_and_a_concession_still_loses() {
    let mut shown = false;
    let mut conceded = false;
    play_looking("platinum-angel-keeps-its-controller-in-at-zero-life-until-it-is-destroyed", |session, views, _| {
        let applies = says(views, 0, 0, "You can't lose the game.");
        assert_eq!(says(views, 1, 0, "They can't lose the game."), applies, "the seats are told different things of Bot1");
        assert_eq!(says(views, 1, 1, "You can't win the game."), applies, "Bot2 is not told it can't win");
        assert_eq!(says(views, 0, 1, "They can't win the game."), applies, "Bot1 is not told Bot2 can't win");
        shown |= applies;
        if applies && views[0].players[0].life == 0 {
            let step = session.concede(1).map_err(|r| r.detail).unwrap();
            assert_eq!(step["Outcome"], "TeamWon");
            assert_eq!(step["WinningTeam"], 2);
            conceded = true;
            return true;
        }
        false
    });
    assert!(shown, "nobody was told of the Angel");
    assert!(conceded, "Bot1 was never at 0 life with the Angel out");
}

/// Phyrexian Unlife's state is shown to both players (CR 104.3b, 702.90b): "You don't lose the game for having 0 or less life."
/// while it is out, and, once Bot1 is at 0 or less life, "Damage is dealt to you as though its source had infect." - each told
/// to Bot2 of Bot1 in its own words. Melira's "You can't get poison counters." likewise.
#[test]
fn phyrexian_unlife_and_melira_are_shown_to_both_players() {
    let (mut unlife, mut infect, mut melira) = (false, false, false);
    play_looking("melira-with-phyrexian-unlife-damage-dealt-as-poison-does-nothing", |_, views, _| {
        let at_most_zero = views[0].players[0].life <= 0;
        let pairs = [
            ("You don't lose the game for having 0 or less life.", "They don't lose the game for having 0 or less life.", &mut unlife),
            ("Damage is dealt to you as though its source had infect.", "Damage is dealt to them as though its source had infect.", &mut infect),
            ("You can't get poison counters.", "They can't get poison counters.", &mut melira),
        ];
        let unlife_out = says(views, 0, 0, pairs[0].0);
        for (own, other, seen) in pairs {
            let applies = says(views, 0, 0, own);
            assert_eq!(says(views, 1, 0, other), applies, "the seats are told different things: {own}");
            *seen |= applies;
        }
        assert_eq!(says(views, 0, 0, "Damage is dealt to you as though its source had infect."), at_most_zero && unlife_out);
        false
    });
    assert!(unlife && infect && melira, "not everything was shown: {unlife} {infect} {melira}");
}

/// The granted lines of the permanent named `name` in `view`, if it is shown.
fn granted_lines(view: &cantrip_host::protocol::PlayerView, name: &str) -> Option<Vec<String>> {
    view.objects.iter().find(|o| o.name == name).map(|o| o.granted_abilities.iter().map(|a| a.text.clone()).collect())
}

/// Protection a permanent was given (CR 702.16) is shown on it to both seats, worded as cards print it: Akroma's Memorial's
/// "protection from black and from red" on Bot1's Hired Blade, and Mother of Runes's "protection from red". Progenitus's own
/// protection from everything is its printed line and is not shown again as granted.
#[test]
fn protection_a_permanent_was_given_is_shown_and_a_printed_one_is_not_repeated() {
    let mut memorial = [false, false];
    play_looking("akromas-memorial-protection-from-red-drops-a-red-aura-and-equipment-and-turns-a-bolt-away", |_, views, _| {
        for (seat, view) in views.iter().enumerate() {
            if granted_lines(view, "Hired Blade").is_some_and(|lines| lines.iter().any(|l| l == "Protection from black and from red")) {
                memorial[seat] = true;
            }
        }
        false
    });
    assert_eq!(memorial, [true, true], "both seats are told the Blade has protection from black and from red");

    let mut mother = false;
    play_looking("mother-of-runes-after-blocks-leaves-the-blocker-blocking-and-prevents-its-damage", |_, views, _| {
        mother |= granted_lines(&views[1], "Hired Blade").is_some_and(|lines| lines.iter().any(|l| l == "Protection from red"));
        false
    });
    assert!(mother, "Bot2 is told the Blade Mother of Runes protected has protection from red");

    // Mother of Runes on herself: her own text says "protection from", and the protection an effect gave her is still shown.
    let mut herself = false;
    play_looking("mother-of-runes-in-response-makes-a-lightning-bolt-fizzle", |_, views, _| {
        herself |= granted_lines(&views[0], "Mother of Runes").is_some_and(|lines| lines.iter().any(|l| l == "Protection from red"));
        false
    });
    assert!(herself, "Bot1 is told Mother of Runes gave herself protection from red");

    let mut progenitus_seen = false;
    play_looking("progenitus-cannot-be-targeted-blocked-enchanted-or-damaged", |_, views, _| {
        for view in views {
            if let Some(lines) = granted_lines(view, "Progenitus") {
                progenitus_seen = true;
                assert!(!lines.iter().any(|l| l.starts_with("Protection")), "Progenitus's printed protection is shown as granted: {lines:?}");
            }
        }
        false
    });
    assert!(progenitus_seen, "the scenario never showed Progenitus");
}

/// A player's protection from a player (CR 702.16k) is shown to both players beside the protected player, as control is now:
/// while Bot1 controls Absolute Virtue, Bot1 is told "You have protection from Bot2." and Bot2 that Bot1 has it; once Bot2's
/// Act of Treason has taken the Virtue, Bot2 is told it has protection from Bot1, and Bot1 is told nothing of its own.
#[test]
fn protection_from_a_player_is_shown_to_both_players_and_follows_control() {
    let (mut bot1_protected, mut bot2_protected) = (false, false);
    play_looking("absolute-virtue-stolen-protects-its-new-controller-and-not-its-owner", |_, views, _| {
        let first = says(views, 0, 0, "You have protection from Bot2.");
        assert_eq!(says(views, 1, 0, "They have protection from Bot2."), first, "the seats are told different things of Bot1");
        let second = says(views, 1, 1, "You have protection from Bot1.");
        assert_eq!(says(views, 0, 1, "They have protection from Bot1."), second, "the seats are told different things of Bot2");
        assert!(!(first && second), "both players were shown protection at once");
        assert!(!(bot2_protected && first), "Bot1's protection came back while Bot2 controlled the Virtue");
        bot1_protected |= first;
        bot2_protected |= second;
        false
    });
    assert!(bot1_protected && bot2_protected, "not both were shown: {bot1_protected} {bot2_protected}");
}

/// "You may look at the top card of your library any time" (CR 401.2, `PlayRule::LookAtLibraryTop`): while Bolas's Citadel is
/// on Bot1's battlefield, Bot1's view holds the top card of Bot1's library, named, whether or not it can be played - Island,
/// on top after the turn's land was played - and Bot2's view holds no card of that library; before the Citadel, neither does.
#[test]
fn bolass_citadel_shows_its_controller_the_top_of_their_library_and_nobody_else() {
    use cantrip_host::protocol::ZoneKindView;
    let mut tops: Vec<String> = Vec::new();
    play_looking("bolass-citadel-plays-off-the-top-for-life", |_, views, _| {
        let citadel = views[0]
            .objects
            .iter()
            .any(|o| o.name == "Bolas's Citadel" && o.zone == Some(ZoneKindView::Battlefield) && o.controller == 1);
        let library = |viewer: usize| -> Vec<&cantrip_host::protocol::ObjectView> {
            views[viewer].objects.iter().filter(|o| o.zone == Some(ZoneKindView::Library) && o.zone_owner == 1).collect()
        };
        assert!(library(1).is_empty(), "Bot2 is shown a card of Bot1's library");
        let seen = library(0);
        if citadel {
            assert_eq!(seen.len(), 1, "Bot1 is not shown exactly its top card under the Citadel");
            assert!(seen[0].known && !seen[0].name.is_empty());
            if tops.last() != Some(&seen[0].name) {
                tops.push(seen[0].name.clone());
            }
        } else {
            assert!(seen.is_empty(), "Bot1 is shown its library without the Citadel");
        }
        false
    });
    assert_eq!(tops, ["Lightning Bolt", "Mox Sapphire", "Island", "Tundra", "Swamp"], "the tops Bot1 was shown");
}

/// The host's `CheckBlocks` (docs/host-protocol.md, "Checking a declaration of blockers") judges Bot1's declaration under
/// Lure (CR 509.1c): no blocks is no declaration, with the issue naming the creature that must block; the scenario's own block
/// is one; and the prompt's verdict on no blocks is the host's.
#[test]
fn the_host_judges_a_declaration_of_blockers_under_lure() {
    let name = "lure-tells-the-defender-which-creature-must-block-in-the-browser";
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).ok().unwrap();
    let mut step = session.step();
    for _ in 0..20_000 {
        let expected = step["Scenario"]["Expected"].clone();
        assert!(!expected.is_null(), "the declaration never came");
        let answer: AnswerPayload = serde_json::from_value(expected["Answer"].clone()).unwrap();
        let id = expected["RequestId"].as_u64().unwrap() as u32;
        let seat = expected["Seat"].as_i64().unwrap() as i32;
        if let AnswerPayload::BlockersDeclaredAnswer(declared) = &answer {
            let none = session.check_blocks(id, seat, &[]).ok().unwrap();
            assert_eq!(none["Complete"], Value::Bool(false));
            assert_eq!(none["Issue"]["Rule"], Value::String("509.1c".into()));
            assert_eq!(none["Issue"]["Words"], Value::String("must block".into()));
            assert_eq!(none["Addable"].as_array().map(Vec::len), Some(1));
            let prompt = step["Requests"].to_string();
            assert!(prompt.contains("\"Verdict\""), "the prompt carries no verdict: {prompt}");
            let theirs = session.check_blocks(id, seat, &declared.blocks).ok().unwrap();
            assert_eq!(theirs["Complete"], Value::Bool(true), "{theirs}");
            assert!(theirs["Issue"].is_null());
            return;
        }
        step = session.answer(id, seat, &answer).ok().unwrap();
    }
    panic!("the declaration never came");
}

/// The host's `CheckAttacks` (docs/host-protocol.md, "Checking a declaration of attackers") counts CR 508.1d's requirements:
/// the goaded Ursine Monstrosity told to attack its goader may attack either opponent, and attacking nothing is refused with
/// the 508.1d issue.
#[test]
fn the_host_judges_a_declaration_of_attackers_under_goad() {
    use cantrip_host::protocol::AttackMessage;
    let name = "a-goaded-ursine-monstrosity-told-to-attack-its-goader-may-attack-bot3";
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut session = Session::scenario(&text, name, Vec::new(), &[1]).ok().unwrap();
    let mut step = session.step();
    // Turns 4 and 7 send it at Bot3 untaxed and ungoaded; turn 10's is the goaded declaration.
    let mut at_bot3 = 0;
    for _ in 0..20_000 {
        let expected = step["Scenario"]["Expected"].clone();
        assert!(!expected.is_null(), "the declaration never came");
        let answer: AnswerPayload = serde_json::from_value(expected["Answer"].clone()).unwrap();
        let id = expected["RequestId"].as_u64().unwrap() as u32;
        let seat = expected["Seat"].as_i64().unwrap() as i32;
        if let AnswerPayload::AttackersDeclaredAnswer(declared) = &answer
            && declared.attacks.first().is_some_and(|a| a.target_player == 3)
            && {
                at_bot3 += 1;
                at_bot3 == 3
            }
        {
            let none = session.check_attacks(id, seat, &[]).ok().unwrap();
            assert_eq!(none["Complete"], Value::Bool(false));
            assert_eq!(none["Issue"]["Rule"], Value::String("508.1d".into()));
            let ursine = declared.attacks[0].attacker;
            for target_player in [2, 3] {
                let attack = AttackMessage { attacker: ursine, target_player, target_permanent: ViewHandle::NONE };
                let verdict = session.check_attacks(id, seat, &[attack]).ok().unwrap();
                assert_eq!(verdict["Complete"], Value::Bool(true), "attacking seat {target_player}: {verdict}");
            }
            let too_many = vec![AttackMessage { attacker: ursine, target_player: 2, target_permanent: ViewHandle::NONE }; 9];
            assert!(session.check_attacks(id, seat, &too_many).is_err());
            return;
        }
        step = session.answer(id, seat, &answer).ok().unwrap();
    }
    panic!("the declaration never came");
}

/// A name question answered with no name (CR 201.3; `Answer::Name(None)`, which the host writes as an empty name) reads
/// back as no name, not as the lookup of the name "" - which a nameless token's card matched, a name the core then refused
/// as no card's (game 57 of the Phase 2 gate duel).
#[test]
fn an_empty_card_name_reads_back_as_no_name() {
    use cantrip_engine::{Answer, NameKind, ObjectId, PlayerId, Request, RequestKind};
    let name = "demonic-consultation-names-a-card-and-exiles-everything-above-it";
    let text = std::fs::read_to_string(scripts().join(format!("{name}.cantrip"))).unwrap();
    let mut scripted = cantrip_script::Scripted::deal(GameScript::parse(&text, name).unwrap()).map_err(|f| f.report).unwrap();
    let game = scripted.game().clone();
    let kind = RequestKind::ChooseName { source: ObjectId(0), looked_at: Vec::new(), suggestion: None, kind: NameKind::ANY };
    let request = Request { id: 1, player: PlayerId(0), kind, may_cancel: false };
    let none: AnswerPayload = serde_json::from_value(serde_json::json!({"$type": "CardNamedAnswer", "Name": ""})).unwrap();
    let read = cantrip_host::answer::translate(&none, &request, &cantrip_host::handles::HandleSpace::new(), &game);
    let read = read.map_err(|e| match e {
        cantrip_host::answer::Untranslatable::Illegal(d) | cantrip_host::answer::Untranslatable::UnknownHandle(d) => d,
    });
    assert!(matches!(read, Ok(Answer::Name(None))), "an empty name reads back as {read:?}");
}
