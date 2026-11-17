use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER, effects::you_mill::<2>, "Whenever this creature deals combat damage to a player, you mill two cards."),
        ability(cost("{1}{B}"), effects::this_gets::<2, 2>, "{1}{B}: This creature gets +2/+2 until end of turn. Activate only once each turn.").once_each_turn(),
    ],
    ..oracle::FESTERLEECH
};
