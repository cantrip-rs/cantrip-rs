use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        lands::SACRIFICE_TO_FETCH_BASIC_TAPPED,
        ability(TAP.and(SACRIFICE_THIS), effects::target_creature_cant_be_blocked_this_turn, "{T}, Sacrifice this land: Target creature with power 2 or less can't be blocked this turn.")
            .targets(&[targets::CREATURE_WITH_POWER_2_OR_LESS]),
    ],
    ..oracle::ESCAPE_TUNNEL
};
