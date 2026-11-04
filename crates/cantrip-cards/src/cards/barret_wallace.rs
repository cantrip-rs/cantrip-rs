use cantrip_api::*;
use cantrip_mechanics::attach::is_equipped;
use cantrip_mechanics::triggers::THIS_ATTACKS_DEFENDER;
use cantrip_oracle as oracle;

/// "It deals damage equal to the number of equipped creatures you control to defending player" (CR 508.5, 301.5a): counted as the
/// ability resolves (the ruling), by whoever controls Barret then; the defending player is the one it attacked, or the controller of the
/// planeswalker or battle it attacked now. Damage of 0 is not dealt (CR 120.8).
fn damage_for_equipped_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(defending) = g.defending_player(item.object) else { return Ok(()) };
    let equipped = g.permanents_of(item.controller).into_iter().filter(|&id| is_equipped(&*g, id)).count();
    g.damage(item.object, Target::Player(defending), equipped as u16);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS_DEFENDER,
        damage_for_equipped_creatures,
        "Whenever Barret Wallace attacks, it deals damage equal to the number of equipped creatures you control to defending player.",
    )],
    ..oracle::BARRET_WALLACE
};
