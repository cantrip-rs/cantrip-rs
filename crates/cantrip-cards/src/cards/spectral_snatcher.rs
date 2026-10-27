use cantrip_api::*;
use cantrip_mechanics::{triggers, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward_discard("Ward\u{2014}Discard a card. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player discards a card.)"),
        typecycling::ability(Subtype::Swamp, mana("{2}"), "Swampcycling {2} ({2}, Discard this card: Search your library for a Swamp card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::SPECTRAL_SNATCHER
};
