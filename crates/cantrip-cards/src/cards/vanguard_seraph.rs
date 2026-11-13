use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// Flying is a core keyword. "Whenever you gain life for the first time each turn, surveil 1" (CR 603.2, 701.25a): the first life
/// gain of each turn, on any player's turn, and only the controller's; a second gain in the same turn does not trigger it.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::LIFE_CHANGE, triggers::you_gain_life_for_the_first_time_each_turn),
        |g, item| effects::surveil(g, item, 1),
        "Whenever you gain life for the first time each turn, surveil 1.",
    )],
    ..oracle::VANGUARD_SERAPH
};
