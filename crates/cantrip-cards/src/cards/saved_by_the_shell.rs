use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counters_on_first_target_that_gains_until_end_of_turn;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast if you control a Turtle" (CR 601.2f): judged as the total cost is determined, once, and
/// generic mana only (CR 118.7a).
fn less_with_a_turtle(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let word = q.words(source).subtype(Subtype::Turtle);
    let turtle = q.permanents_of(player).into_iter().any(|id| q.chars(id).has_subtype(word));
    if matches!(costed, Costed::Spell(_)) && turtle { CostDelta::less(1) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE_YOU_CONTROL],
        plus_one_counters_on_first_target_that_gains_until_end_of_turn::<
            1,
            { Keywords::of(&[Keyword::Trample, Keyword::Hexproof, Keyword::Indestructible]).0 },
        >,
    ),
    own_cost_changes: &[less_with_a_turtle],
    ..oracle::SAVED_BY_THE_SHELL
};
