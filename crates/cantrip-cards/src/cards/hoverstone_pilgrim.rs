use cantrip_api::*;
use cantrip_mechanics::{library, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        ability(cost("{2}"), library::put_first_target_on_the_bottom, "{2}: Put target card from a graveyard on the bottom of its owner's library.")
            .targets(&[targets::CARD_IN_A_GRAVEYARD]),
    ],
    ..oracle::HOVERSTONE_PILGRIM
};
