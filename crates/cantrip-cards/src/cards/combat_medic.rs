use cantrip_api::*;
use cantrip_mechanics::{prevention, targets};
use cantrip_oracle as oracle;

/// Prevent the next 1 damage to the target this turn (CR 615.7).
fn shield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::next_damage_to_target(g, item, 0, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{W}"),
        shield,
        "{1}{W}: Prevent the next 1 damage that would be dealt to any target this turn.",
    )
    .targets(&[targets::ANY])],
    granted_preventions: &[prevention::ANY_DAMAGE
        .text("Prevent the next 1 damage that would be dealt to any target this turn.")],
    ..oracle::COMBAT_MEDIC
};
