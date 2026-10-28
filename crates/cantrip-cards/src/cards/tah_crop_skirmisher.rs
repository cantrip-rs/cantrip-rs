use cantrip_api::*;
use cantrip_mechanics::embalm;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        embalm::ability(
            mana("{3}{U}"),
            "Embalm {3}{U} ({3}{U}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a white Zombie Snake Warrior with no mana cost. Embalm only as a sorcery.)",
        ),
    ],
    ..oracle::TAH_CROP_SKIRMISHER
};
