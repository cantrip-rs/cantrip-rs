use cantrip_api::*;
use cantrip_mechanics::{lands, statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "Other Allies you control have menace and prowess" (CR 613.1f, 702.111a, 702.108a): one static ability per granted ability, both
/// on the same set of creatures; each other Ally gets a prowess ability of its own (CR 113.2c, `statics::grant_delayed_ability`),
/// so it triggers separately from Sokka's.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::PROWESS,
        triggered(triggers::NONCREATURE_SPELL_YOU_CAST, tokens::create_an_ally, "Whenever you cast a noncreature spell, create a 1/1 white Ally creature token."),
    ],
    delayed: &[triggers::PROWESS],
    statics: &[
        static_ability(
            Layer::Ability,
            statics::other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Ally) }>,
            statics::have_menace,
            "Other Allies you control have menace.",
        ),
        static_ability(
            Layer::Ability,
            statics::other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Ally) }>,
            statics::grant_delayed_ability,
            "Other Allies you control have prowess. (If a creature has multiple instances of prowess, each triggers separately.)",
        ),
    ],
    ..oracle::SOKKA_TENACIOUS_TACTICIAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
