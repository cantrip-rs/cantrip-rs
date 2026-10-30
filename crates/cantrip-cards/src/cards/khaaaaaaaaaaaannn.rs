use cantrip_api::*;
use cantrip_mechanics::replacements::{EXILE_INSTEAD_OF_DYING, exile_if_it_would_die};
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Khaaaaaaaaaaaannn! deals twice X damage to target creature": the X announced as it is cast (CR 601.2b), read when the
/// spell resolves (CR 608.2h) and doubled, so the damage is twice that number (CR 107.3a). A target that has become
/// illegal is dealt nothing (CR 608.2b), and the replacement still applies to nothing.
///
/// "If that creature would die this turn, exile it instead": the replacement is the spell's own effect on that creature
/// for the turn (CR 611.2c, 614.1a), made before the damage so that the state-based actions see it.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    exile_if_it_would_die(g, item, 0, creature);
    // Doubled in u32 so a large X cannot overflow; damage is a u16 (RX-5), so the product is held at its largest value.
    let damage = u16::try_from(u32::from(item.x) * 2).unwrap_or(u16::MAX);
    damage_target(g, item, 0, damage);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING],
    ..oracle::KHAAAAAAAAAAAANNN
};
