use cantrip_api::*;
use cantrip_mechanics::unearth;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        unearth::ability(mana("{2}"), "Unearth {2}"),
    ],
    delayed: &[unearth::EXILE],
    ..oracle::RECONSTRUCTED_THOPTER
};
