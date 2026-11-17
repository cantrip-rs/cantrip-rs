use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{3}, {T}"), effects::first_target_cant_block_this_turn, "{3}, {T}: Target creature can't block this turn. Activate only as a sorcery.")
            .targets(&[targets::CREATURE])
            .sorcery_speed(),
    ],
    ..oracle::SANDSTORM_VERGE
};
