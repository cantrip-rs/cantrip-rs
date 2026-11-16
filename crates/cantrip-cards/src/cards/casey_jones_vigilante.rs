use cantrip_api::*;
use cantrip_mechanics::discard_at_random;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When Casey Jones enters, draw three cards. At the beginning of your next upkeep, discard three cards at random."
/// The second sentence sets up a delayed triggered ability (CR 603.7) for the first upkeep of its controller's that has not
/// begun yet; it is independent of Casey Jones from then on (CR 603.7e: "your" is the controller of the ability that created
/// it), so it happens if he has left the battlefield. The ability is the one on Casey's card that is resolving, whatever the object has become (CR 707.2).
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 3);
    g.delayed_trigger(
        DelayedWhen::Step { step: Step::Upkeep, on_turn_of: Some(item.controller) },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

/// "Discard three cards at random" (CR 701.9a-b): the player chooses nothing, and a smaller hand is discarded whole.
fn discard_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discard_at_random(g, item.controller, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, enters, "When Casey Jones enters, draw three cards. At the beginning of your next upkeep, discard three cards at random.")],
    delayed: &[delayed_trigger(discard_three, "At the beginning of your next upkeep, discard three cards at random.")],
    ..oracle::CASEY_JONES_VIGILANTE
};
