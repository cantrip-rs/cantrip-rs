use cantrip_api::*;
use cantrip_mechanics::{prevention, targets};
use cantrip_oracle as oracle;

/// Prevent the next two damage to the target creature this turn (CR 615.7, 608.2b).
fn prevent_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::next_damage_to_target(g, item, 0, 0, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, prevent_two, "{T}: Prevent the next 2 damage that would be dealt to target creature this turn.")
        .targets(&[targets::CREATURE])],
    granted_preventions: &[prevention::ANY_DAMAGE.text("Prevent the next 2 damage that would be dealt to target creature this turn.")],
    ..oracle::KEI_TAKAHASHI
};
