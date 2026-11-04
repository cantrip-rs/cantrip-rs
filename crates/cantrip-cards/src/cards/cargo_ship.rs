use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP,
            add(Mana::C).only_for(ManaRestriction::ARTIFACT_SPELLS_OR_ABILITIES),
            "{T}: Add {C}. Spend this mana only to cast an artifact spell or activate an ability of an artifact source.",
        ),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::CARGO_SHIP
};
