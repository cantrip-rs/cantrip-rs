use cantrip_api::*;
use cantrip_mechanics::spectacle;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[spectacle::way(mana("{2}{R}"), "Spectacle {2}{R}")],
    ..oracle::SPIKEWHEEL_ACROBAT
};
