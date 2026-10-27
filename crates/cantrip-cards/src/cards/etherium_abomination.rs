use cantrip_api::*;
use cantrip_mechanics::unearth;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        unearth::ability(mana("{1}{U}{B}"), "Unearth {1}{U}{B}"),
    ],
    delayed: &[unearth::EXILE],
    ..oracle::ETHERIUM_ABOMINATION
};
