use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature gains 'When this creature dies, return it to the battlefield tapped under its owner's control
/// with a +1/+1 counter on it'" (CR 611.2a, 613.1f): the card's own delayed ability for the rest of the turn. With the target gone
/// nothing is gained (CR 608.2b).
fn gain_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let this = g.card_id(&CARD);
    g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
    Ok(())
}

/// "Return it to the battlefield tapped under its owner's control with a +1/+1 counter on it" (CR 110.2a, 400.7, 122.6): the card as
/// it is in the graveyard (each card of a merged permanent, together, CR 730.3c); a token is not a card and has ceased to exist, so
/// there is nothing to return.
fn return_it_with_a_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_what_it_became(g, item.object, true, &[(CounterKind::PlusOne, 1)])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], gain_return),
    delayed: &[triggered(
        THIS_DIES,
        return_it_with_a_counter,
        "When this creature dies, return it to the battlefield tapped under its owner's control with a +1/+1 counter on it.",
    )],
    ..oracle::UNDYING_MALICE
};
