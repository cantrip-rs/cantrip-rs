use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        // "Spend this mana only to cast a Lesson or Shrine spell" (CR 106.6, 205.3): spells only, not the abilities of Shrines
        // (the ruling of 2025-10-02).
        mana_ability(
            TAP,
            add_any_color(1).only_for(ManaRestriction::Subtypes(SubtypeSpells::of_any(&[Subtype::Lesson, Subtype::Shrine]))),
            "{T}: Add one mana of any color. Spend this mana only to cast a Lesson or Shrine spell.",
        ),
        lands::ANY_COLOR_FOR_ONE,
    ],
    ..oracle::WHITE_LOTUS_HIDEOUT
};
