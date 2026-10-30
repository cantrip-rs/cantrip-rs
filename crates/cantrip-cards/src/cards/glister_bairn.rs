use cantrip_api::*;
use cantrip_mechanics::{pump, targets, triggers::BEGINNING_OF_YOUR_COMBAT, vivid};
use cantrip_oracle as oracle;

/// "Another target creature you control gets +X/+X until end of turn, where X is the number of colors among permanents you
/// control": X is worked out once, as the ability resolves.
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = i16::from(vivid::count(g, item.controller));
    pump(g, item.object, creature, x, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        grow,
        "Vivid — At the beginning of combat on your turn, another target creature you control gets +X/+X until end of turn, where X is the number of colors among permanents you control.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::GLISTER_BAIRN
};
