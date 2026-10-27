use cantrip_api::*;
use cantrip_mechanics::{triggers, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<3>("Ward {3} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {3}.)"),
        typecycling::ability(Subtype::Island, mana("{2}"), "Islandcycling {2} ({2}, Discard this card: Search your library for an Island card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::MARAUDING_BRINEFANG
};
