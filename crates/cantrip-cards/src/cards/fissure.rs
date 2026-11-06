use cantrip_api::*;
use cantrip_mechanics::{destroy_target_no_regen, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature or land. It can't be regenerated." (CR 701.8a, 701.19c).
fn fissure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target_no_regen(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_LAND], fissure),
    ..oracle::FISSURE
};
