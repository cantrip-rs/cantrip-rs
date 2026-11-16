use cantrip_api::*;
use cantrip_mechanics::{effects, statics, targets, triggers};
use cantrip_oracle as oracle;

/// "Prowess" and "other creatures you control have prowess" (CR 702.108a, 113.6, 613.1f): each other creature gets a prowess
/// ability of its own (CR 113.2c, `statics::grant_delayed_ability`), so it triggers separately from Bria's.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::PROWESS,
        triggered(
            triggers::NONCREATURE_SPELL_YOU_CAST,
            effects::target_creature_cant_be_blocked_this_turn,
            "Whenever you cast a noncreature spell, target creature you control can't be blocked this turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    delayed: &[triggers::PROWESS],
    statics: &[static_ability(
        Layer::Ability,
        statics::other_creatures_you_control,
        statics::grant_delayed_ability,
        "Other creatures you control have prowess. (If a creature has multiple instances of prowess, each triggers separately.)",
    )],
    ..oracle::BRIA_RIPTIDE_ROGUE
};
