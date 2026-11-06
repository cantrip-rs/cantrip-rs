//! Refuse one answer to a number question without refusing the question itself.
use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::choose;
use cantrip_script::{GameScript, ScriptFailure, run_on};

fn choose_number(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let n = choose::number(g, item.controller, item.object, 2, 4, "Choose a number")?;
    g.gain_life(item.controller, n as i32);
    Ok(())
}

static PROBE: CardDef = CardDef {
    name: "Probe Number",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], choose_number),
    ..CardDef::BLANK
};
static FILLER: CardDef = CardDef {
    name: "Number Filler",
    oracle: true,
    types: TypeSet::of(&[CardType::Land]),
    ..CardDef::BLANK
};

fn play(refusals: &str) -> Result<(), ScriptFailure> {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new([&PROBE, &FILLER]));
    let text = format!(
        "[Bot1 Deck]\nProbe Number\nNumber Filler x19\n\n[Bot2 Deck]\nNumber Filler x20\n\n[Actions]\nBot1.Hand.Probe Number.Cast\nBot1.Pass\nBot2.Pass\n{refusals}\nBot1.Number(3)\n[T1 Bot1.Main2]\n\n[Expect]\nBot1.Life = 23\n"
    );
    run_on(&GameScript::parse(&text, "number-refusal").unwrap(), table).map(|_| ())
}

#[test]
fn refusing_values_outside_the_range_leaves_the_same_question_to_answer() {
    play("! Bot1.Number(1)\n! Bot1.Number(5)").unwrap();
}

#[test]
fn refusing_either_endpoint_or_an_interior_value_fails() {
    for n in [2, 3, 4] {
        let failure = play(&format!("! Bot1.Number({n})")).expect_err("the value is legal");
        assert!(failure.message.contains("must not be allowed"), "{}", failure.report);
    }
}

#[test]
fn a_bare_refusal_still_refuses_the_whole_question() {
    let failure = play("! Bot1.Number").expect_err("a number was asked");
    assert!(failure.message.contains("asked that question"), "{}", failure.report);
}

#[test]
fn a_malformed_refused_number_is_an_error() {
    let failure = play("! Bot1.Number(two)").expect_err("malformed number");
    assert!(failure.message.contains("not a number"), "{}", failure.report);
}
