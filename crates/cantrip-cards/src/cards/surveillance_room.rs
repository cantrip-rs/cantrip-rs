use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        lands::SURVEIL_LAND,
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        lands::ANY_COLOR_FOR_ONE,
    ],
    ..oracle::SURVEILLANCE_ROOM
};
