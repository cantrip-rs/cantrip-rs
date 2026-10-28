use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "PayForModes",
    "pay {1} up to three times?",
    "When you do, choose up to that many: target creature can't block this turn, Hawkeye deals 2 damage to target player, discard a card then draw a card",
    "Pay {cost}",
    "Don't pay",
)
.no_default();

/// "Whenever Hawkeye becomes tapped, you may pay {1} up to three times. When you do, choose up to that many" (CR 118.12, 608.2d,
/// 603.12, 603.12a): one payment whose number of times is chosen as it is paid, and a reflexive ability that triggers once for any
/// number of times above none, counting them.
fn becomes_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let paid = g.may_pay_times(you, item.object, item.object, mana("{1}"), 3, &PAY)?;
    if paid > 0 {
        let this = g.card_id(&CARD);
        g.reflexive_trigger_counted(item.object, you, AbilityRef::delayed(this, 0), Target::None, paid);
    }
    Ok(())
}

/// "Explosive \u{2014} Hawkeye deals 2 damage to target player."
fn explosive(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 2);
    Ok(())
}

/// "Boomerang \u{2014} Discard a card, then draw a card."
fn boomerang(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::discard_then_draw(g, item, 1)
}

/// The reflexive ability's modes, as many as {1} was paid times at most (CR 603.12a, 700.2b), chosen with their targets as it is put
/// on the stack (CR 603.3c) and carried out in the order printed (CR 700.2d).
static MODAL: ModalDef = modes(&[
    mode(effects::first_target_cant_block_this_turn, "Net \u{2014} Target creature can't block this turn.").targets(&[targets::CREATURE]),
    mode(explosive, "Explosive \u{2014} Hawkeye deals 2 damage to target player.").targets(&[targets::PLAYER]),
    mode(boomerang, "Boomerang \u{2014} Discard a card, then draw a card."),
])
.up_to_that_many();

/// Reach and first strike are core keywords.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_BECOMES_TAPPED,
        becomes_tapped,
        "Trick Arrows \u{2014} Whenever Hawkeye becomes tapped, you may pay {1} up to three times. When you do, choose up to that many \u{2014} Net \u{2014} Target creature can't block this turn; Explosive \u{2014} Hawkeye deals 2 damage to target player; Boomerang \u{2014} Discard a card, then draw a card.",
    )],
    delayed: &[modal_delayed_trigger(
        &MODAL,
        "When you pay this cost one or more times, choose up to that many \u{2014} Net \u{2014} Target creature can't block this turn; Explosive \u{2014} Hawkeye deals 2 damage to target player; Boomerang \u{2014} Discard a card, then draw a card.",
    )],
    ..oracle::HAWKEYE_MASTER_MARKSMAN
};
