use cantrip_api::*;
use cantrip_mechanics::tokens::{self, FOOD};
use cantrip_mechanics::triggers::{THIS_ENTERS, YOUR_END_STEP};
use cantrip_mechanics::{effects, history, targets};
use cantrip_oracle as oracle;

/// "Up to one other target artifact or creature" (CR 115.1, 205.2a): a permanent that is either, not Pizza Face itself.
fn other_artifact_or_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    t != Target::Object(asker.source) && targets::artifact_or_creature_filter(q, asker, t)
}

/// "Put three +1/+1 counters on it. If it isn't a creature, it becomes a 0/0 Mutant creature in addition to its other types."
fn three_counters_and_becomes_a_mutant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::three_counters_and_becomes(g, item, Subtype::Mutant)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| tokens::create_food(g, item.controller), "When Pizza Face enters, create a Food token."),
        triggered(
            YOUR_END_STEP,
            three_counters_and_becomes_a_mutant,
            "Disappear \u{2014} At the beginning of your end step, if a permanent left the battlefield under your control this turn, put three +1/+1 counters on up to one other target artifact or creature. If it isn't a creature, it becomes a 0/0 Mutant creature in addition to its other types.",
        )
        .condition(history::disappear)
        .targets(&[up_to_one(other_artifact_or_creature)]),
        ability(
            cost("{10}, {T}").and(SACRIFICE_THIS),
            |g, item| effects::gain_life(g, item, 15),
            "{10}, {T}, Sacrifice Pizza Face: You gain 15 life.",
        ),
    ],
    ..oracle::PIZZA_FACE_GASTROMANCER
};

/// The Food token it makes.
pub static EXTRA: &[&CardDef] = &[&FOOD];
