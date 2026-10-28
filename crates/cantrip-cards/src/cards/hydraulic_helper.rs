use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This mana can't be spent to cast a nonartifact spell" (CR 106.6): it pays for artifact spells and for abilities.
    abilities: &[mana_ability(
        TAP,
        add(Mana::U).only_for(ManaRestriction::NotNonartifactSpells),
        "{T}: Add {U}. This mana can't be spent to cast a nonartifact spell.",
    )],
    ..oracle::HYDRAULIC_HELPER
};
