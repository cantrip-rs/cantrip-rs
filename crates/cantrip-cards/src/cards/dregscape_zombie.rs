use cantrip_api::*;
use cantrip_mechanics::unearth;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        unearth::ability(mana("{B}"), "Unearth {B}"),
    ],
    delayed: &[unearth::EXILE],
    ..oracle::DREGSCAPE_ZOMBIE
};
