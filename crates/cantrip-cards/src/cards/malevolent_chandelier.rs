use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}"),
        library::put_first_target_on_the_bottom,
        "{2}: Put target card from a graveyard on the bottom of its owner's library. Activate only as a sorcery.",
    )
    .targets(&[targets::CARD_IN_A_GRAVEYARD])
    .sorcery_speed()],
    ..oracle::MALEVOLENT_CHANDELIER
};
