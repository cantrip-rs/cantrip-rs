use cantrip_api::*;
use cantrip_mechanics::{eternalize, ferocious};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        mana_ability(TAP, add_n(Mana::G, 4), "Ferocious \u{2014} {T}: Add {G}{G}{G}{G}. Activate only if you control a creature with power 4 or greater.")
            .condition(ferocious::you_control_asked),
        eternalize::ability(
            mana("{2}{G}{G}"),
            "Eternalize {2}{G}{G} ({2}{G}{G}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a 4/4 black Zombie Snake Druid with no mana cost. Eternalize only as a sorcery.)",
        ),
    ],
    ..oracle::FANATIC_OF_RHONAS
};
