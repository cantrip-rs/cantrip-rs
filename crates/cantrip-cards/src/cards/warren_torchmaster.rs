use cantrip_api::*;
use cantrip_mechanics::{blight, haste_until_end_of_turn, targets, triggers};
use cantrip_oracle as oracle;

const BLIGHT: Question = Question::new("Blight", "blight 1 to give a creature haste?", "When you do, target creature gains haste until end of turn", "Blight 1", "Don't");

/// "You may blight 1. When you do, ..." (CR 701.68a, 118.12, 603.12): the cost is paid as the trigger resolves, and only if it
/// was paid does the reflexive ability trigger and choose its target then.
fn blight_for_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if g.may_pay(you, item.object, item.object, blight::cost(1), &BLIGHT)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, you, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

/// Target creature gains haste until end of turn, if it is still a legal target (CR 608.2b).
fn haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    haste_until_end_of_turn(g, item.object, creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        blight_for_haste,
        "At the beginning of combat on your turn, you may blight 1. When you do, target creature gains haste until end of turn.",
    )],
    delayed: &[delayed_trigger(haste, "When you do, target creature gains haste until end of turn.").targets(&[targets::CREATURE])],
    ..oracle::WARREN_TORCHMASTER
};
