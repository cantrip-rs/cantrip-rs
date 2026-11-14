use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
        crew(3, "Crew 3 (Tap any number of creatures you control with total power 3 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::CULTIVATORS_CARAVAN
};
