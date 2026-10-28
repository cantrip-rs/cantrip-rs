use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// "Target creature you control that's attacking alone gets +1/+0 until end of turn. You gain 1 life." (CR 611.2a, 119.3): the
/// ability does nothing at all if its target has stopped being a legal one - left combat, or no longer attacking alone - by the
/// time it resolves (CR 608.2b), and then the life is not gained either.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, creature, 1, 0);
    g.end_sentence();
    g.gain_life(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, effect, "{T}: Target creature you control that's attacking alone gets +1/+0 until end of turn. You gain 1 life.")
        .targets(&[targets::CREATURE_YOU_CONTROL_ATTACKING_ALONE])],
    ..oracle::CROWD_OF_TRUE_BELIEVERS
};
