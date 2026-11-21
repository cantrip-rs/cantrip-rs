use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "Return it to the battlefield tapped under its owner's control with a +1/+1 counter on it" (CR 110.2a, 400.7, 122.6): the card as
/// it is in the graveyard (each card of a merged permanent, together, CR 730.3c); a token is not a card and has ceased to exist, so
/// there is nothing to return.
fn return_it_with_a_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_what_it_became(g, item.object, true, &[(CounterKind::PlusOne, 1)])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], delayed::first_target_gains_its_delayed_ability_until_end_of_turn),
    delayed: &[triggered(
        THIS_DIES,
        return_it_with_a_counter,
        "When this creature dies, return it to the battlefield tapped under its owner's control with a +1/+1 counter on it.",
    )],
    ..oracle::UNDYING_MALICE
};
