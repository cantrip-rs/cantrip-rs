use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// A suspected creature (CR 701.60a); it is sacrificed as a cost (CR 601.2h), so no response can stop the -5/-5 by removing it.
fn suspected_creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.is_suspected(id)
}

/// "When this creature enters, suspect up to one target creature you control" (CR 701.60a): the target is a creature of the controller's,
/// not any creature. "{3}{B}{R}, Sacrifice a suspected creature: Target creature gets -5/-5 until end of turn" (CR 611.2a): the sacrificed
/// creature may be any suspected creature the controller has, this one included, and the target is chosen before the cost is paid.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::suspect_target::<0>,
            "When this creature enters, suspect up to one target creature you control. (A suspected creature has menace and can't block.)",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL.optional()]),
        ability(
            cost("{3}{B}{R}").and(sacrifice(1, suspected_creature)),
            effects::target_creature_gets::<-5, -5>,
            "{3}{B}{R}, Sacrifice a suspected creature: Target creature gets -5/-5 until end of turn.",
        )
        .targets(&[targets::CREATURE]),
    ],
    ..oracle::RUNE_BRAND_JUGGLER
};
