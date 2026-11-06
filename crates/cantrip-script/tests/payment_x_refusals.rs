//! An X refusal concerns the choice of X, rather than the current PayCost button.
//! Probing another X must leave the actual resolution and payment unchanged.
use std::sync::OnceLock;
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, ScriptFailure, run_on};

const ASK: Question = Question::new("Pay", "Pay {cost}?", "", "Pay", "Decline");
fn fixed(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::C, 3);
    if g.may_pay(item.controller, item.object, item.object, of_mana(mana("{1}")), &ASK)? {
        g.gain_life(item.controller, 1);
    }
    Ok(())
}
fn variable(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::C, 3);
    if let Some(x) = g.may_pay_x(item.controller, item.object, item.object, of_mana(mana("{X}")), &ASK)? {
        g.gain_life(item.controller, i32::try_from(x).expect("test payment fits life"));
    }
    Ok(())
}
fn times(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::C, 3);
    let n = g.may_pay_times(item.controller, item.object, item.object, mana("{1}"), 3, &ASK)?;
    g.gain_life(item.controller, i32::try_from(n).expect("test payment fits life"));
    Ok(())
}
/// A deliberately small test picker, so the harness must obey its actual options rather than assume the core's usual
/// X range. Its free payment records the chosen X as life; this tests the harness, not a new printed-card procedure.
fn bounded(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let mut x = 1;
    loop {
        let mut options = vec![ActionOption::simple(ActionKind::PayCost), ActionOption::simple(ActionKind::DeclineCost)];
        if x < 2 { options.push(ActionOption::simple(ActionKind::IncreaseX)); }
        if x > 0 { options.push(ActionOption::simple(ActionKind::DecreaseX)); }
        let answer = g.ask(item.controller, RequestKind::UnlessPays {
            source: item.object, subject: item.object, total: ManaCost::ZERO, remaining: ManaCost::ZERO,
            life: 0, life_payable: true, question: &ASK, options, x: Some(x), times: None,
        })?;
        match answer {
            Answer::Act(action) if action.kind == ActionKind::IncreaseX => x += 1,
            Answer::Act(action) if action.kind == ActionKind::DecreaseX => x -= 1,
            Answer::Act(action) if action.kind == ActionKind::PayCost => { g.gain_life(item.controller, i32::try_from(x).expect("test payment fits life")); return Ok(()); }
            Answer::Act(action) if action.kind == ActionKind::DeclineCost => return Ok(()),
            _ => unreachable!("only the offered picker actions are answers"),
        }
    }
}
static FIXED: CardDef = CardDef {
    name: "Fixed Payment", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], fixed), ..CardDef::BLANK
};
static VARIABLE: CardDef = CardDef { name: "Variable Payment", spell: spell(&[], variable), ..FIXED };
static TIMES: CardDef = CardDef { name: "Repeated Payment", spell: spell(&[], times), ..FIXED };
static BOUNDED: CardDef = CardDef { name: "Bounded Payment", spell: spell(&[], bounded), ..FIXED };
static FILLER: CardDef = CardDef { name: "Payment Filler", oracle: true, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn play(name: &str, actions: &str, life: i32) -> Result<(), ScriptFailure> {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new([&FIXED, &VARIABLE, &TIMES, &BOUNDED, &FILLER]));
    let text = format!("[Bot1 Deck]\n{name}\nPayment Filler x19\n[Bot2 Deck]\nPayment Filler x20\n[Actions]\nBot1.Hand.{name}.Cast\nBot1.Pass\nBot2.Pass\n{actions}\n[T1 Bot1.Main2]\n[Expect]\nBot1.Life = {life}\n");
    run_on(&GameScript::parse(&text, "payment-x-refusal").unwrap(), table).map(|_| ())
}

#[test]
fn a_fixed_payment_has_no_x_choice_even_though_paying_is_offered() {
    play("Fixed Payment", "! Bot1.PayCost(X: 1)\n! Bot1.PayCost(X: 0)\nBot1.PayCost", 21).unwrap();
}
#[test]
fn refusing_a_legal_x_fails_even_when_the_pool_cannot_pay_that_amount() {
    for x in [0, 1, 3, 4] {
        let failure = play("Variable Payment", &format!("! Bot1.PayCost(X: {x})\nBot1.PayCost"), 20).expect_err("X is choosable");
        assert!(failure.message.contains("must not"), "{}", failure.report);
    }
}
#[test]
fn an_unreachable_x_refusal_preserves_the_original_question() {
    // The fork may reach 2, but the real payment remains at its initial X=1.
    play("Bounded Payment", "! Bot1.PayCost(X: 3)\nBot1.PayCost", 21).unwrap();
}
#[test]
fn a_bounded_pickers_endpoints_and_current_value_are_legal() {
    for x in [0, 1, 2] {
        let failure = play("Bounded Payment", &format!("! Bot1.PayCost(X: {x})\nBot1.PayCost"), 21).expect_err("X is choosable");
        assert!(failure.message.contains("must not"), "{}", failure.report);
    }
}
#[test]
fn an_x_refusal_does_not_mistake_a_number_of_times_for_x() {
    let failure = play("Repeated Payment", "! Bot1.PayCost(X: 4)\nBot1.PayCost", 21).expect_err("wrong question");
    assert!(failure.message.contains("number of times"), "{}", failure.report);
}
#[test]
fn a_malformed_refused_x_is_an_error() {
    let failure = play("Fixed Payment", "! Bot1.PayCost(X: two)\nBot1.PayCost", 21).expect_err("malformed X");
    assert!(failure.message.contains("not a number"), "{}", failure.report);
}
#[test]
fn an_explicit_x_cannot_be_silently_ignored_on_an_ordinary_payment() {
    let failure = play("Fixed Payment", "Bot1.PayCost(X: 1)", 21).expect_err("no X choice");
    assert!(failure.message.contains("does not choose X"), "{}", failure.report);
}
#[test]
fn a_bare_refusal_still_checks_the_payment_option_itself() {
    let failure = play("Fixed Payment", "! Bot1.PayCost\nBot1.PayCost", 21).expect_err("payment is offered");
    assert!(failure.message.contains("must not be offered"), "{}", failure.report);
}
