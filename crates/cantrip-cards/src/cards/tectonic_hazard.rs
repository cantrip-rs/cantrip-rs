use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Tectonic Hazard deals 1 damage to each opponent and each creature they control" (CR 120.2, 608.2c): one damage event for
/// each opponent still in the game and each creature one controls, all dealt together.
fn one_apiece(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent_and_each_creature_they_control(g, item, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], one_apiece), ..oracle::TECTONIC_HAZARD };
