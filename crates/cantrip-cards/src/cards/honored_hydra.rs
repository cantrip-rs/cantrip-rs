use cantrip_api::*;
use cantrip_mechanics::embalm;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        embalm::ability(
            mana("{3}{G}"),
            "Embalm {3}{G} ({3}{G}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a white Zombie Snake Hydra with no mana cost. Embalm only as a sorcery.)",
        ),
    ],
    ..oracle::HONORED_HYDRA
};
