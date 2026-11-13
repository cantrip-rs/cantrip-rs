use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add(Mana::U).only_for(ManaRestriction::ARTIFACT_SPELLS_OR_ANY_ABILITY),
        "{T}: Add {U}. Spend this mana only to cast an artifact spell or activate an ability.",
    )],
    ..oracle::GUIDELIGHT_OPTIMIZER
};
