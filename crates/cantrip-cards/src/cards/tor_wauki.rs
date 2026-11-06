use cantrip_api::*;
use cantrip_mechanics::{damage_each_target, targets};
use cantrip_oracle as oracle;

/// "{T}: Tor Wauki deals 2 damage to target attacking or blocking creature." (CR 120.3, 506.4).
fn deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_each_target(g, item, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        deal_damage,
        "{T}: Tor Wauki deals 2 damage to target attacking or blocking creature.",
    )
    .targets(&[targets::ATTACKING_OR_BLOCKING_CREATURE])],
    ..oracle::TOR_WAUKI
};
