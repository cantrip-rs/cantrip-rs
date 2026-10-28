use cantrip_api::*;
use cantrip_mechanics::triggers::ward;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        typecycling::basic_landcycling(mana("{2}"), "Basic landcycling {2} ({2}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::HUMPBACK_WHALES
};
