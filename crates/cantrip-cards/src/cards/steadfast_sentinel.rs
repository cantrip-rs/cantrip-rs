use cantrip_api::*;
use cantrip_mechanics::eternalize;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        eternalize::ability(mana("{4}{W}{W}"), "Eternalize {4}{W}{W} ({4}{W}{W}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a 4/4 black Zombie Human Cleric with no mana cost. Eternalize only as a sorcery.)"),
    ],
    ..oracle::STEADFAST_SENTINEL
};
