use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add(Mana::U).only_for(ManaRestriction::ENCHANTMENT_SPELLS_OR_UNLOCKING_DOORS_OR_TURNING_UP),
        "{T}: Add {U}. Spend this mana only to cast an enchantment spell, unlock a door, or turn a permanent face up.",
    )],
    ..oracle::CREEPING_PEEPER
};
