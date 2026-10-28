use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::ENCHANTED_CREATURES_CONTROLLERS_UPKEEP};
use cantrip_oracle as oracle;

/// "That player draws a card": the controller of the enchanted creature whose upkeep it is, the trigger's subject (not
/// the Aura's controller).
fn that_player_draws(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(player) = item.subject {
        g.draw(player, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        ENCHANTED_CREATURES_CONTROLLERS_UPKEEP,
        that_player_draws,
        "At the beginning of the upkeep of enchanted creature's controller, that player draws a card.",
    )],
    ..oracle::SUPER_INTELLIGENCE
};
