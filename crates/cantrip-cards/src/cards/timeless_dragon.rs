use cantrip_api::*;
use cantrip_mechanics::{eternalize, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        typecycling::ability(Subtype::Plains, mana("{2}"), "Plainscycling {2} ({2}, Discard this card: Search your library for a Plains card, reveal it, put it into your hand, then shuffle.)"),
        eternalize::ability(mana("{2}{W}{W}"), "Eternalize {2}{W}{W} ({2}{W}{W}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a 4/4 black Zombie Dragon with no mana cost. Eternalize only as a sorcery.)"),
    ],
    ..oracle::TIMELESS_DRAGON
};
