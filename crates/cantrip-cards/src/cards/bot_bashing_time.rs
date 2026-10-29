use cantrip_api::*;
use cantrip_mechanics::replacements::{EXILE_INSTEAD_OF_DYING, exile_if_it_would_die};
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Bot Bashing Time deals 6 damage to target creature. If that creature would die this turn, exile it instead": the
/// replacement is the spell's own effect on that creature for the turn (CR 611.2c, 614.1a), as Wilt in the Heat's is.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    exile_if_it_would_die(g, item.object, 0, creature);
    damage_target(g, item, 0, 6);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING],
    ..oracle::BOT_BASHING_TIME
};
