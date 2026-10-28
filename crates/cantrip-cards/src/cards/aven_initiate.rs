use cantrip_api::*;
use cantrip_mechanics::embalm;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        embalm::ability(
            mana("{6}{U}"),
            "Embalm {6}{U} ({6}{U}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a white Zombie Bird Warrior with no mana cost. Embalm only as a sorcery.)",
        ),
    ],
    ..oracle::AVEN_INITIATE
};
