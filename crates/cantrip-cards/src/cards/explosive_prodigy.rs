use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets, triggers};
use cantrip_oracle as oracle;

/// Vivid (CR 207.2c): "it deals X damage to target creature an opponent controls, where X is the number of colors among
/// permanents you control" (CR 105.2). X is counted as the ability resolves (CR 608.2h), so a permanent that has left, or a
/// color gained since, counts as it is then; the damage is dealt by this creature, last-known if it has left (CR 608.2b).
fn deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = cantrip_mechanics::vivid::count(g, item.controller) as u16;
    damage_target(g, item, 0, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, deal_damage, "Vivid \u{2014} When this creature enters, it deals X damage to target creature an opponent controls, where X is the number of colors among permanents you control.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::EXPLOSIVE_PRODIGY
};
