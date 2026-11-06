use cantrip_engine::{Event, PlayerId, RequestKind, ZoneKind};
use cantrip_script::{Advance, GameScript, Scripted};

fn script(actions: &str) -> String {
    format!(r#"[Bot1 Deck]
Mountain
Mox Ruby
Mox Emerald
Lightning Bolt
Lightning Bolt
Island x15

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Mountain.Play
Bot1.Hand.Mox Ruby.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Emerald.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Mox Ruby.Activate
{actions}
"#)
}

fn run_both(name: &str, text: &str) {
    let s = GameScript::parse(text, name).unwrap();
    let alone = cantrip_script::run(&s).unwrap_or_else(|e| panic!("{}", e.report));
    let expected = alone.dump.render();
    let mut driven = Scripted::deal(s).unwrap();
    loop {
        match driven.advance(&[PlayerId(0), PlayerId(1)]).unwrap_or_else(|e| panic!("{}", e.report)) {
            Advance::Ask(ask) => driven.answered(ask.intent).unwrap_or_else(|e| panic!("{}", e.report)),
            Advance::Check(_) => {},
            Advance::Done => break,
        }
    }
    assert_eq!(driven.verdict().unwrap().render(), expected);
}

#[test]
fn funded_cast_finishes_before_checks_and_next_mana_ability() {
    run_both("covered-then-next-mana", &script(r#"Bot1.Hand.Lightning Bolt.Cast(Bot2)
? Bot1.Mana =
? Stack.Count = 1
Bot1.Battlefield.Mox Emerald.Activate
? Bot1.Mana = {G}
? Bot1.ManaFrom = {G} Mox Emerald
Bot1.Battlefield.Mountain.Activate
Bot1.Hand.Lightning Bolt.Cast(Bot2)
? Bot1.Mana = {G}
? Bot1.ManaFrom = {G} Mox Emerald
? Stack.Count = 2
"#));
}

#[test]
fn explicit_close_keeps_extra_mana_activation_before_payment() {
    run_both("manual-mana-window", &script(r#"Bot1.Hand.Lightning Bolt.Cast(Bot2)
Bot1.Battlefield.Mox Emerald.Activate
? Bot1.Mana = {R}{G}
Bot1.Choose(Close mana window)
? Bot1.Mana = {G}
? Bot1.ManaFrom = {G} Mox Emerald
? Stack.Count = 1
"#));
}

#[test]
fn explicit_cancel_rolls_back_extra_mana_ability_and_cast() {
    run_both("cancel-mana-window", &script(r#"Bot1.Hand.Lightning Bolt.Cast(Bot2)
Bot1.Battlefield.Mox Emerald.Activate
? Bot1.Battlefield.Mox Emerald.Tapped = true
Bot1.Cancel
? Bot1.Battlefield.Mox Emerald.Tapped = false
? Bot1.Mana = {R}
? Bot1.ManaFrom = {R} Mox Ruby
? Bot1.Hand.Count = 4
? Stack.Count = 0
"#));
}

#[test]
fn handover_keeps_the_covered_mana_window_pending() {
    let s = GameScript::parse(&script(r#"Bot1.Hand.Lightning Bolt.Cast(Bot2)
[Handover]
"#), "covered-window-handover").unwrap();
    let result = cantrip_script::run(&s).unwrap_or_else(|e| panic!("{}", e.report));
    assert!(result.handed_over);
    assert!(matches!(result.game.pending().unwrap().kind, RequestKind::PayCosts { mana_abilities: true, .. }));
    let expected = result.dump.render();
    let mut driven = Scripted::deal(s).unwrap();
    loop {
        match driven.advance(&[PlayerId(0), PlayerId(1)]).unwrap() {
            Advance::Ask(ask) => driven.answered(ask.intent).unwrap(),
            Advance::Check(_) => {},
            Advance::Done => break,
        }
    }
    assert!(matches!(driven.game().pending().unwrap().kind, RequestKind::PayCosts { mana_abilities: true, .. }));
    assert_eq!(driven.verdict().unwrap().render(), expected);
}

#[test]
fn undo_restores_a_mana_activation_inside_the_window() {
    run_both("undo-mana-window", &script(r#"Bot1.Hand.Lightning Bolt.Cast(Bot2)
Bot1.Battlefield.Mox Emerald.Activate
? Bot1.Battlefield.Mox Emerald.Tapped = true
Bot1.Undo
? Bot1.Battlefield.Mox Emerald.Tapped = false
? Bot1.Mana = {R}
Bot1.Choose(Close mana window)
? Bot1.Mana =
? Stack.Count = 1
"#));
}

#[test]
fn explicit_cost_order_preserves_both_legal_payment_sequences() {
    let original = r#"# Embrace Oblivion sacrifices Bot1's Mox Ruby (chosen over the Ornithopter, either is an artifact) and destroys Bot1's own
# Ornithopter: one of your own is a legal target.

[Bot1 Deck]
Mox Ruby
Swamp
Ornithopter
Embrace Oblivion
Swamp x16

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Swamp.Play
Bot1.Hand.Ornithopter.Cast
Bot1.Battlefield.Swamp.Activate
Bot1.Hand.Embrace Oblivion.Cast(Bot1.Battlefield.Ornithopter)
Bot1.Choose(Pay mana)
Bot1.Choose(Mox Ruby)
Bot1.Pass

[Expect]
Bot1.Graveyard = Embrace Oblivion; Ornithopter; Mox Ruby
"#;
    for (name, choice, mana_first) in [
        ("mana-first", "Bot1.Choose(Pay mana)", true),
        ("sacrifice-first", "Bot1.Choose(Sacrifice 1 permanent)", false),
    ] {
        let text = original.replace("Bot1.Choose(Pay mana)", choice);
        run_both(name, &text);
        let s = GameScript::parse(&text, name).unwrap();
        let result = cantrip_script::run(&s).unwrap_or_else(|e| panic!("{}", e.report));
        let mut mana = None;
        let mut sacrifice = None;
        for (i, record) in result.game.log.iter().enumerate() {
            match record.event {
                Event::ManaSpent { cast: true, .. } => mana = Some(i),
                Event::ZoneChange { old, from, to, .. }
                    if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard)
                        && result.game.known_name(old) == "Mox Ruby" => sacrifice = Some(i),
                _ => {},
            }
        }
        assert_eq!(mana.expect("mana spent") < sacrifice.expect("Mox Ruby sacrificed"), mana_first);
    }
}

#[test]
fn attack_payment_refuses_the_tapped_attacker_and_can_be_cancelled() {
    let original = r#"# The chosen attackers become tapped (CR 508.1f) before any cost to attack is determined and paid (CR 508.1h-j), so a
# creature cannot tap for mana to pay its own way past Ghostly Prison. On turn 3 Bot1's only mana is the Llanowar Elves
# it declares as an attacker: while the {2} is owed its mana ability is not offered, nothing can pay, and Bot1 gives the
# declaration up. The Elves untap with it (CR 733), and the declaration made again is no attack at all: with no mana a
# taxed attack cannot be made. Bot2 stays at 20 and the Elves end untapped.

[Bot1 Deck]
Lotus Petal
Llanowar Elves
Island
Island
Island
Island
Island
Island x10

[Bot2 Deck]
Plains
Black Lotus
Ghostly Prison
Plains
Plains
Plains
Plains
Plains x10

[Actions]
Bot1.Hand.Lotus Petal.Cast
Bot1.Battlefield.Lotus Petal.Activate(Color: Green)
Bot1.Hand.Llanowar Elves.Cast
[T2 Bot2.Main1]
Bot2.Hand.Black Lotus.Cast
Bot2.Battlefield.Black Lotus.Activate(Color: White)
Bot2.Hand.Ghostly Prison.Cast
[T3 Bot1.Main1]
Bot1.Attack(Llanowar Elves)
? Combat.AttackCost = {2}
! Bot1.Battlefield.Llanowar Elves.Activate
Bot1.Cancel
[T3 Bot1.Main2]
? Bot1.Battlefield.Llanowar Elves.Tapped = false
Bot1.Battlefield.Llanowar Elves.Activate
? Bot1.Mana = {G}

[Expect]
Turn = 3
Bot2.Life = 20
Bot1.Battlefield.Llanowar Elves.Tapped = true
Combat.Attackers.Count = 0
"#;
    run_both("attack-tax-refusal", original);
    let illegal = original.replace("! Bot1.Battlefield.Llanowar Elves.Activate", "Bot1.Battlefield.Llanowar Elves.Activate");
    let script = GameScript::parse(&illegal, "attack-tax-illegal-mana").unwrap();
    assert!(cantrip_script::run(&script).is_err(), "a tapped attacker cannot pay its own attack cost");
}

#[test]
fn implicit_mana_first_finishes_before_a_post_action_check() {
    let original = r#"# Airship Crash's cycling {2} (CR 702.29a): paying it with Mana Crypt's {C}{C} exactly discards the card from the hand and draws a
# card, the Forest on top, and the pool ends empty. Casting it needs the {G} of {2}{G}: with Mox Ruby, Mox Jet and Mox Pearl
# untapped and no green source the cast, even at its own Mox Ruby, is not offered for want of mana (`Mana: short`).

[Bot1 Deck]
Mana Crypt
Mox Ruby
Mox Jet
Mox Pearl
Airship Crash
Airship Crash
Island
Forest
Island x15

[Bot2 Deck]
Island x24

[Actions]
Bot1.Hand.Mana Crypt.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Pearl.Cast
! Bot1.Hand.Airship Crash.Cast(Bot1.Battlefield.Mox Ruby; Mana: short)
Bot1.Battlefield.Mana Crypt.Activate
Bot1.Hand.Airship Crash.Activate
? Bot1.Mana =
Bot1.Pass

[Expect]
Bot1.Graveyard = Airship Crash
Bot1.Hand = Airship Crash; Forest; Island
"#;
    run_both("cycling-mana-first", original);
    let manual = original.replace("Bot1.Hand.Airship Crash.Activate\n", "Bot1.Hand.Airship Crash.Activate\nBot1.Choose(Pay mana)\n? Bot1.Mana = {C}\nBot1.Choose(Pay mana)\n");
    run_both("cycling-manual-intermediate-mana", &manual);
}

#[test]
fn mana_ability_is_not_offered_after_the_player_starts_paying() {
    let text = r#"[Bot1 Deck]
Mountain
Mox Ruby
Mox Jet
Mox Emerald
Ornithopter
Fling
Island
Island x15

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Mountain.Play
Bot1.Hand.Mox Ruby.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Emerald.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Ornithopter.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Hand.Fling.Cast(Bot2)
Bot1.Choose(Pay mana)
! Bot1.Battlefield.Mountain.Activate
Bot1.Cancel
? Bot1.Battlefield.Mountain.Tapped = false
? Bot1.Mana = {B}{R}{G}
? Stack.Count = 0
"#;
    run_both("mana-after-payment-started", text);
    let mut driven = Scripted::deal(GameScript::parse(text, "mana-after-payment-boundary").unwrap()).unwrap();
    loop {
        match driven.advance(&[PlayerId(0), PlayerId(1)]).unwrap() {
            Advance::Ask(ask) if ask.line.as_deref().is_some_and(|line| line.contains("Bot1.Cancel")) => {
                assert!(matches!(ask.request.kind, RequestKind::PayCosts { mana_abilities: false, .. }));
                break;
            }
            Advance::Ask(ask) => driven.answered(ask.intent).unwrap(),
            Advance::Check(_) => {},
            Advance::Done => panic!("the cancellation question was not reached"),
        }
    }
    let illegal = text.replace("! Bot1.Battlefield.Mountain.Activate", "Bot1.Battlefield.Mountain.Activate");
    let script = GameScript::parse(&illegal, "mana-after-payment-illegal").unwrap();
    assert!(cantrip_script::run(&script).is_err(), "mana abilities cannot be activated after cost payment starts");
}

#[test]
fn covered_payment_completes_across_special_actions_and_step_boundaries() {
    let future_refusal = script(r#"Bot1.Hand.Lightning Bolt.Cast(Bot2)
? Bot1.Mana =
? Stack.Count = 1
! Bot1.Battlefield.Mox Ruby.Activate
"#);
    for (name, source) in [
        ("face-up-handover", include_str!("../../../scenarios/abhorrent-oculus-manifested-creature-card-turns-face-up-for-its-mana-cost.cantrip")),
        ("face-up-explicit-assign", include_str!("../../../scenarios/dress-down-does-not-stop-a-manifested-creature-turning-face-up.cantrip")),
        ("attack-tax-step", include_str!("../../../scenarios/embercleave-counts-the-attackers-paid-for-under-ghostly-prison.cantrip")),
        ("effect-cast", include_str!("../../../scenarios/doc-aurlock-does-not-reduce-a-spell-cast-from-an-opponents-graveyard.cantrip")),
        ("miracle-cast", include_str!("../../../scenarios/molecule-man-and-a-cards-own-miracle-the-player-may-take-its-own.cantrip")),
        ("future-refusal", future_refusal.as_str()),
        ("future-refusal-on-mana-ability", include_str!("../../../scenarios/foraging-wickermaw-becomes-the-color-of-its-mana-for-the-turn-once-a-turn.cantrip")),
    ] {
        run_both(name, source);
    }
}

#[test]
fn explicitly_open_window_still_rejects_false_refusal_after_a_check() {
    let text = script(r#"Bot1.Hand.Lightning Bolt.Cast(Bot2)
? Bot1.Mana = {R}
! Bot1.Battlefield.Mountain.Activate
Bot1.Choose(Close mana window)
"#);
    let s = GameScript::parse(&text, "manual-window-false-refusal").unwrap();
    let failure = cantrip_script::run(&s).err().expect("the untapped Mountain is offered in the mana window");
    assert!(failure.message.contains("must not be offered, but it is"), "{}", failure.report);
}

#[test]
fn named_object_choice_uses_unambiguous_mana_before_its_question() {
    let text = include_str!("../../../scenarios/embrace-oblivion-can-destroy-its-controllers-own-creature-when-another-permanent-is-sacrificed.cantrip");
    assert!(!text.contains("Bot1.Choose(Pay mana)"));
    run_both("named-object-after-mana", text);
}

#[test]
fn two_nonmana_components_are_not_guessed_from_a_later_object() {
    let original = include_str!("../../../scenarios/springleaf-drum-needs-an-untapped-creature-of-your-own.cantrip");
    let text = original.replace("Bot1.Choose(Tap Springleaf Drum)\n", "");
    let s = GameScript::parse(&text, "ambiguous-drum-cost").unwrap();
    let failure = cantrip_script::run(&s).err().expect("two distinct tap instructions require an explicit order");
    assert!(failure.message.contains("does not identify exactly one payment") || failure.message.contains("payment order is ambiguous"), "{}", failure.report);
}

#[test]
fn refused_effect_choice_is_not_a_cost_order_question() {
    let text = include_str!("../../../scenarios/eerie-gravestone-with-no-creature-among-the-four-milled-asks-nothing.cantrip");
    assert!(text.contains("! Bot1.Choose"));
    run_both("refused-effect-choice-after-payment", text);
}

#[test]
fn source_names_do_not_hide_distinct_additional_costs() {
    let text = include_str!("../../../scenarios/fling-casualty-caster-orders-the-costs.cantrip");
    run_both("explicit-additional-cost-sources", text);
}

#[test]
fn refused_explicit_cost_number_checks_the_actual_payment_menu() {
    let original = include_str!("../../../scenarios/springleaf-drum-needs-an-untapped-creature-of-your-own.cantrip");
    let text = original.replace("Bot1.Choose(Tap Springleaf Drum)", "! Bot1.Choose(Cost: 1)\nBot1.Choose(Tap Springleaf Drum)");
    let script = GameScript::parse(&text, "refused-offered-cost").unwrap();
    let failure = cantrip_script::run(&script).err().expect("the first cost is offered");
    assert!(failure.message.contains("must not be offered, but it is"), "{}", failure.report);
}

#[test]
fn refused_explicit_cost_label_checks_the_actual_payment_menu() {
    let original = include_str!("../../../scenarios/springleaf-drum-needs-an-untapped-creature-of-your-own.cantrip");
    let text = original.replace("Bot1.Choose(Tap Springleaf Drum)", "! Bot1.Choose(Tap Springleaf Drum)\nBot1.Choose(Tap Springleaf Drum)");
    let script = GameScript::parse(&text, "refused-offered-cost-label").unwrap();
    let failure = cantrip_script::run(&script).err().expect("the tap cost is offered");
    assert!(failure.message.contains("must not be offered, but it is"), "{}", failure.report);
}
