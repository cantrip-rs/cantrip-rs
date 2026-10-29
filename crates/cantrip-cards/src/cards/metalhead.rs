use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Up to one other target artifact or creature": any permanent but Metalhead itself (an artifact creature) that is one.
fn other_artifact_or_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::artifact_or_creature_filter(q, asker, t) && t != Target::Object(asker.source)
}

/// "Put a +1/+1 counter on Metalhead. He gains menace and haste until end of turn." (CR 122.1, 611.2a, 702.110, 702.10)
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    g.end_sentence();
    g.one_instruction(&mut |g| {
        effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Menace);
        effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Haste);
        Ok(())
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::return_first_target_to_hand, "When Metalhead enters, return up to one other target artifact or creature to its owner's hand.")
            .targets(&[up_to_one(other_artifact_or_creature)]),
        ability(
            cost("{R}").and(sacrifice(1, targets::filters::another_artifact)),
            grow,
            "{R}, Sacrifice another artifact: Put a +1/+1 counter on Metalhead. He gains menace and haste until end of turn.",
        ),
    ],
    ..oracle::METALHEAD
};
