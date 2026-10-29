use cantrip_api::*;
use cantrip_mechanics::replacements::{EXILE_INSTEAD_OF_DYING, exile_if_it_would_die};
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Feed the Flames deals 5 damage to target creature. If that creature would die this turn, exile it instead." The replacement is
/// the spell's own effect on that creature for the turn (CR 611.2c, 614.1a), for any way it would die, made whether or not the
/// damage is dealt, as Combustion Technique's is.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    exile_if_it_would_die(g, item.object, 0, creature);
    damage_target(g, item, 0, 5);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING],
    ..oracle::FEED_THE_FLAMES
};
