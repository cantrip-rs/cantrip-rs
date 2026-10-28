use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::U).only_for(ManaRestriction::INSTANT_OR_SORCERY_SPELLS), "{T}: Add {U}. Spend this mana only to cast an instant or sorcery spell."),
        mana_ability(cost("{1}, {T}"), add_any_color(1).only_for(ManaRestriction::INSTANT_OR_SORCERY_SPELLS), "{1}, {T}: Add one mana of any color. Spend this mana only to cast an instant or sorcery spell."),
    ],
    ..oracle::HYDRO_CHANNELER
};
