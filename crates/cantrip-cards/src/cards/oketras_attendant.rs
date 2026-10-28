use cantrip_api::*;
use cantrip_mechanics::{embalm, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)"),
        embalm::ability(
            mana("{3}{W}{W}"),
            "Embalm {3}{W}{W} ({3}{W}{W}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a white Zombie Bird Soldier with no mana cost. Embalm only as a sorcery.)",
        ),
    ],
    ..oracle::OKETRAS_ATTENDANT
};
