use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{2}, {T}: Each opponent loses 1 life and you gain 1 life" (CR 119.3): the {T} cost needs the creature to have been under its
/// controller's control since their turn began (CR 302.6).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}").and(TAP),
        effects::each_opponent_loses_1_life_you_gain_1,
        "{2}, {T}: Each opponent loses 1 life and you gain 1 life.",
    )],
    ..oracle::VAMPIRE_NEONATE
};
