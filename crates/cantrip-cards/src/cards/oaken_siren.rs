use cantrip_api::*;
use cantrip_oracle as oracle;

// Flying and vigilance are the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add(Mana::U).only_for(ManaRestriction::ARTIFACT_SPELLS_OR_ABILITIES),
        "{T}: Add {U}. Spend this mana only to cast an artifact spell or activate an ability of an artifact source.",
    )],
    ..oracle::OAKEN_SIREN
};
