use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST_WITH_FOUR_MANA_SPENT;
use cantrip_oracle as oracle;

/// Haste is in the characteristics. "Selfie Shot - Whenever you cast a noncreature spell, if at least four mana was spent to cast it,
/// create a Treasure token." (CR 603.4, 111.10a): the mana spent is the total paid, whatever its kind and whatever the spell's mana
/// value; it resolves before the spell that caused it.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        NONCREATURE_SPELL_YOU_CAST_WITH_FOUR_MANA_SPENT,
        tokens::create_a_treasure,
        "Selfie Shot \u{2014} Whenever you cast a noncreature spell, if at least four mana was spent to cast it, create a Treasure token.",
    )],
    ..oracle::PROMPTO_ARGENTUM
};
