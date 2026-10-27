use cantrip_api::*;
use cantrip_mechanics::unearth;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        unearth::ability(mana("{2}{U}"), "Unearth {2}{U}"),
    ],
    delayed: &[unearth::EXILE],
    ..oracle::KATHARI_SCREECHER
};
