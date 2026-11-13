use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "This creature deals 1 damage to each creature with flying and each player." (CR 120.3, 608.2h).
fn damage_flyers_and_players(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature_with_flying_and_each_player(g, item, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{G}"),
        damage_flyers_and_players,
        "{G}: This creature deals 1 damage to each creature with flying and each player. Any player may activate this ability.",
    ).any_player_may_activate()],
    ..oracle::IFH_B_FF_EFREET
};
