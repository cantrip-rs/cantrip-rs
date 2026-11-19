use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, you may sacrifice an artifact. When you do, ..." (CR 701.21a, 603.12): the artifact (this creature
/// is one) is chosen as the ability resolves, and the reflexive ability, with its target chosen as it goes on the stack, exists
/// only if one was sacrificed.
fn sacrifice_for_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_sacrifice_when_you_do(g, item, select::artifact_cards(select::up_to(1)))
}

/// "Put a +1/+1 counter on target creature and draw a card." If the target is illegal on resolution the ability does not
/// resolve at all (CR 608.2b), so nothing is drawn either.
fn counter_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_target(g, item)?;
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, sacrifice_for_counter, "When this creature enters, you may sacrifice an artifact. When you do, put a +1/+1 counter on target creature and draw a card.")],
    delayed: &[delayed_trigger(counter_and_draw, "When you do, put a +1/+1 counter on target creature and draw a card.").targets(&[targets::CREATURE])],
    ..oracle::SELFCRAFT_MECHAN
};
