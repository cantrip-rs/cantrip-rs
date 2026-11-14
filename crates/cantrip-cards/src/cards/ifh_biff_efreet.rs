use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Deal 1 damage to each creature with flying and each player simultaneously (CR 120.3, 120.4).
fn damage_flyers_and_players(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut recipients: Vec<Target> = g.state().battlefield().iter().copied()
        .filter(|&id| targets::has_flying_creature(g, id))
        .map(Target::Object).collect();
    recipients.extend(g.state().apnap().map(Target::Player));
    let damage: Vec<_> = recipients.into_iter().map(|target| (item.object, target, 1)).collect();
    g.damage_together(&damage);
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
