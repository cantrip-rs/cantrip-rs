use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

/// "Defending player loses 1 life and you gain 1 life" (CR 508.5, 119.3): the player the Assassin attacked, or the controller of
/// the planeswalker or protector of the battle it attacked, from the attack it made (`Query::defending_player`), also once it has
/// left combat. The gain does not depend on the loss.
fn defending_player_loses_1_life_you_gain_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(defending) = g.defending_player(item.object) {
        g.lose_life(defending, 1);
    }
    g.gain_life(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        defending_player_loses_1_life_you_gain_1,
        "Whenever this creature attacks, defending player loses 1 life and you gain 1 life.",
    )],
    ..oracle::AGATE_BLADE_ASSASSIN
};
