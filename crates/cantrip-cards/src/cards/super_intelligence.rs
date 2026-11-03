use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP};
use cantrip_oracle as oracle;

/// The player whose upkeep triggered the ability, recorded before any change of the enchanted creature's controller.
fn that_player_draws(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(PlayerId(item.data[0] as u8), 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP,
        that_player_draws,
        "At the beginning of the upkeep of enchanted creature's controller, that player draws a card.",
    )],
    ..oracle::SUPER_INTELLIGENCE
};
