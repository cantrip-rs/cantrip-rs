use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Choose up to one other target creature you control. Its base power and toughness become equal to Galion's power and
/// toughness until end of turn." (CR 613.4b, 611.2c, 608.2h): a layer 7b effect whose values are Galion's power and toughness
/// as the ability resolves - his actual ones, counters and effects included, and as he last was if he has left - fixed then, so
/// a later change to him does not touch the creature (the ruling); counters and pumps on the creature still apply on top.
fn copy_size(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let galion = g.chars(item.object);
    let (Some(power), Some(toughness)) = (galion.power, galion.toughness) else { return Ok(()) };
    g.add_effect(item.object, creature, EffectKind::SetPt(power, toughness), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        copy_size,
        "Whenever Galion attacks, choose up to one other target creature you control. Its base power and toughness become equal to Galion's power and toughness until end of turn.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL.optional()])],
    ..oracle::GALION_ELVENKINGS_BUTLER
};
