use cantrip_api::*;
use cantrip_mechanics::{
    combat::{self, EXCEPT_BY_HASTE},
    effects,
};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}"), combat::this_cant_be_blocked_this_turn_except_by_haste, "{1}: This creature can't be blocked this turn except by creatures with haste."),
        ability(
            cost("{2}, {T}").and(SACRIFICE_THIS),
            |g, item| effects::gain_life(g, item, 3),
            "{2}, {T}, Sacrifice this creature: You gain 3 life.",
        ),
    ],
    lasting_block_rules: &[EXCEPT_BY_HASTE],
    ..oracle::GINGERBRUTE
};
