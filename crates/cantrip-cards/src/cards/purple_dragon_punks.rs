use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add(Mana::R).only_for(ManaRestriction::ARTIFACT_SPELLS_OR_ANY_ABILITY),
        "{T}: Add {R}. Spend this mana only to cast an artifact spell or to activate an ability.",
    )],
    ..oracle::PURPLE_DRAGON_PUNKS
};
