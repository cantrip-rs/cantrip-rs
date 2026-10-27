use cantrip_api::*;
use cantrip_mechanics::eternalize;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        eternalize::ability(mana("{3}{W}{W}"), "Eternalize {3}{W}{W} ({3}{W}{W}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a 4/4 black Zombie Cat with no mana cost. Eternalize only as a sorcery.)"),
    ],
    ..oracle::ADORNED_POUNCER
};
