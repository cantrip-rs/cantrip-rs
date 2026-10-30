use cantrip_api::*;
use cantrip_mechanics::chosen_type::enters_choosing_elemental_elf_faerie_giant_goblin_kithkin_merfolk_or_treefolk;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(enters_choosing_elemental_elf_faerie_giant_goblin_kithkin_merfolk_or_treefolk),
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        // "Of the chosen type" is the type chosen as it entered, or none if none was (a copy that never entered): then the
        // mana pays for nothing (CR 607.2a, 106.6).
        mana_ability(
            TAP,
            add_any_color(1).only_for(ManaRestriction::Subtypes(SubtypeSpells::CHOSEN_TYPE_SPELLS_OR_ABILITIES)),
            "{T}: Add one mana of any color. Spend this mana only to cast a spell of the chosen type or activate an ability of a source of the chosen type.",
        ),
    ],
    ..oracle::ECLIPSED_REALMS
};
