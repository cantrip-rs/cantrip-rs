use cantrip_api::*;
use cantrip_mechanics::{delayed, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

const KEEP_EXILED: Question = Question::new("Pay", "pay {cost} so {subject} stays exiled?", "", "Pay {cost}", "Return it");

/// "Exile up to one other target creature. At the beginning of the next end step, you may pay {3}{B}. If you don't, return
/// that card to the battlefield under its owner's control" (CR 603.7): a delayed ability about the card exiled. A token
/// exiled this way ceases to exist, so the card the ability finds is gone (CR 111.7, 400.7).
fn exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile(creature);
    delayed::at_next_end_step(g, item, 0, Target::Object(exiled));
    Ok(())
}

/// "You may pay {3}{B}. If you don't, return that card to the battlefield under its owner's control": asked of the
/// controller of the ability when the end step begins, whether Koya is still on the battlefield or not.
fn pay_or_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(card) = item.subject else { return Ok(()) };
    if !g.may_pay(item.controller, item.object, card, Cost { mana: mana("{3}{B}"), ..Cost::NONE }, &KEEP_EXILED)? {
        delayed::return_subject_to_battlefield(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        exile,
        "When Koya enters, exile up to one other target creature. At the beginning of the next end step, you may pay {3}{B}. If you don't, return that card to the battlefield under its owner's control.",
    )
    .targets(&[targets::OTHER_CREATURE.optional()])],
    delayed: &[delayed_trigger(pay_or_return, "You may pay {3}{B}. If you don't, return that card to the battlefield under its owner's control.")],
    ..oracle::KOYA_DEATH_FROM_ABOVE
};
