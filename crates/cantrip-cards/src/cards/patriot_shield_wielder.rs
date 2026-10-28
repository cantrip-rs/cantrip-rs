use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "Another target creature you control gets +2/+0 and gains hexproof until end of turn." Both are continuous effects of the
/// resolving ability on the creature (CR 611.2a), made at once; an ability whose target has become illegal does nothing
/// (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 2, 0);
        effects::gain_hexproof_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}, {T}"), effect, "{2}, {T}: Another target creature you control gets +2/+0 and gains hexproof until end of turn.")
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::PATRIOT_SHIELD_WIELDER
};
