use cantrip_api::*;
use cantrip_mechanics::embalm;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        embalm::ability(
            mana("{4}{W}"),
            "Embalm {4}{W} ({4}{W}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a white Zombie Human Warrior with no mana cost. Embalm only as a sorcery.)",
        ),
    ],
    ..oracle::UNWAVERING_INITIATE
};
