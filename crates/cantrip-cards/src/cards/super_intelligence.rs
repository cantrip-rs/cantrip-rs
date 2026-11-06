use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST};
use cantrip_oracle as oracle;

/// "That player draws a card": the controller of the enchanted creature whose upkeep it is, saved when the trigger fires (not
/// the Aura's controller).
fn that_player_draws(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = PlayerId(item.data[0] as u8);
    if g.state().in_game(player) {
        g.draw(player, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
        that_player_draws,
        "At the beginning of the upkeep of enchanted creature's controller, that player draws a card.",
    )],
    ..oracle::SUPER_INTELLIGENCE
};
