use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature gets +X/+0 until end of turn." (CR 608.2, 611.2a).
fn target_creature_gets_x_0(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        cantrip_mechanics::pump(g, item.object, creature, item.x as i16, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], target_creature_gets_x_0),
    ..oracle::HOWL_FROM_BEYOND
};
