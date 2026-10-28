use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "This creature gains lifelink until end of turn." Nothing if it has left the battlefield by the time the ability
/// resolves (CR 400.7): the effect is added to the object it was when the ability was put on the stack.
fn lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_lifelink_until_end_of_turn(g, item.object, item.object);
    Ok(())
}

/// "Put a +1/+1 counter on target creature you control." No target left (CR 608.2b) does nothing.
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::plus_one_counter_on_target(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(discard_cards(1), lifelink, "Discard a card: This creature gains lifelink until end of turn."),
        triggered(
            triggers::CARDS_LEAVE_YOUR_GRAVEYARD,
            counter,
            "Whenever one or more cards leave your graveyard, put a +1/+1 counter on target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL])
        .once_per_batch(),
    ],
    ..oracle::HARDENED_ACADEMIC
};
