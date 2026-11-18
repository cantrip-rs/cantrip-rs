use cantrip_api::*;
use cantrip_mechanics::{disguise, effects, targets, triggers};
use cantrip_oracle as oracle;

/// Disguise {4}{R} (CR 702.168). "When this creature enters or is turned face up, up to one target creature gets +2/+0 until end of
/// turn" (CR 603.6a, 701.40b, 611.2a): one ability with two events, so a creature cast face down triggers when it is turned up, not
/// when it enters (CR 708.2). The trigger goes on the stack with no target when none is chosen, and the target may be any creature,
/// this one included.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS_OR_TURNED_FACE_UP,
        effects::target_creature_gets::<2, 0>,
        "When this creature enters or is turned face up, up to one target creature gets +2/+0 until end of turn.",
    )
    .targets(&[targets::CREATURE.optional()])],
    ways: &[disguise::way(Cost::mana(mana("{4}{R}")), "Disguise {4}{R}")],
    ..oracle::OFFENDER_AT_LARGE
};
