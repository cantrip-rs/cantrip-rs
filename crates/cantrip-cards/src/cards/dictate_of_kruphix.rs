use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of each player's draw step, that player draws an additional card." (CR 121.1, 504.1, 603.2): the player whose draw
/// step began, whoever controls the enchantment.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::STEP_BEGAN, triggers::each_players_draw_step),
        effects::that_player_draws_an_additional_card,
        "At the beginning of each player's draw step, that player draws an additional card.",
    )],
    ..oracle::DICTATE_OF_KRUPHIX
};
