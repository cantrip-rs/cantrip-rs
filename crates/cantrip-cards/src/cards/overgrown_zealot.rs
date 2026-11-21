use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
        mana_ability(
            TAP,
            add_any_color(2).only_for(ManaRestriction::TURN_FACE_UP),
            "{T}: Add two mana of any one color. Spend this mana only to turn permanents face up.",
        ),
    ],
    ..oracle::OVERGROWN_ZEALOT
};
