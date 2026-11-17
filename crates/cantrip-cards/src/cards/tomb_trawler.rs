use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}"), library::put_first_target_on_the_bottom, "{2}: Put target card from your graveyard on the bottom of your library.")
        .targets(&[targets::CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::TOMB_TRAWLER
};
