use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Check for a snow Swamp as the ability resolves, then deal damage simultaneously (CR 608.2h, 120.3).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let swamp = g.words(item.object).subtype(Subtype::Swamp);
    let snow_swamp = g.permanents_of(item.controller).into_iter().any(|id| {
        let chars = g.chars(id);
        chars.supertypes.snow && chars.has_subtype(swamp)
    });
    effects::damage_each_creature_and_player_n(g, item, if snow_swamp { 2 } else { 1 });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(SACRIFICE_THIS),
        effect,
        "{T}, Sacrifice this creature: This creature deals 1 damage to each creature and each player. If you control a snow Swamp, this creature deals 2 damage to each creature and each player instead.",
    )],
    ..oracle::GANGRENOUS_ZOMBIES
};
