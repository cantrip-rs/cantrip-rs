use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Reach is a core keyword.
    abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        // "Spend this mana only to cast Mount or Vehicle spells" (CR 106.6, 205.3): a spell with either subtype as cast.
        mana_ability(
            TAP,
            add_any_color(2).only_for(ManaRestriction::Subtypes(SubtypeSpells::of_any(&[Subtype::Mount, Subtype::Vehicle]))),
            "{T}: Add two mana of any one color. Spend this mana only to cast Mount or Vehicle spells.",
        ),
    ],
    ..oracle::INTREPID_STABLEMASTER
};
