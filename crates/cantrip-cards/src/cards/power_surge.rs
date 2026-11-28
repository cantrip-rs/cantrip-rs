use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::EACH_PLAYERS_UPKEEP};
use cantrip_oracle as oracle;

/// "This enchantment deals X damage to that player, where X is the number of untapped lands they controlled at the
/// beginning of this turn": the count the turn's beginning recorded (`History::untapped_lands_as_turn_began`, CR 502.1,
/// 502.3), whatever has become of those lands since - and known even if this enchantment was not on the battlefield then
/// (its ruling). "That player" is the one whose upkeep it is, the player whose turn it is.
fn damage_for_untapped_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(x) = item.subject.player().and_then(|player| g.history().untapped_lands_as_turn_began(player)) else { return Ok(()) };
    effects::damage_that_player(g, item, x)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        EACH_PLAYERS_UPKEEP,
        damage_for_untapped_lands,
        "At the beginning of each player's upkeep, this enchantment deals X damage to that player, where X is the number of untapped lands they controlled at the beginning of this turn.",
    )],
    ..oracle::POWER_SURGE
};
