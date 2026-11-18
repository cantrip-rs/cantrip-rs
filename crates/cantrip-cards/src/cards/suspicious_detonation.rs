use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "This spell costs {3} less to cast if you've sacrificed an artifact this turn" (CR 601.2f, 118.7a, 701.21a): asked of the event
/// history, so an artifact sacrificed earlier this turn counts however it has fared since - a token or a Clue too, judged as it last
/// was on the battlefield - and the reduction applies once however many there were. One the caster did not control as it left, or
/// that left any other way than being sacrificed, does not count. A generic reduction takes only the generic part (CR 118.7a).
fn less_if_you_sacrificed_an_artifact_this_turn(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let sacrificed = q
        .history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).this_turn())
        .any(|r| triggers::sacrificed_by(q, player, &r.event).is_some_and(|old| q.chars(old).is(CardType::Artifact)));
    if sacrificed { CostDelta::less(3) } else { CostDelta::NONE }
}

// "This spell can't be countered" (CR 101.2) is the core keyword `CantBeCountered`.
pub static CARD: CardDef = CardDef {
    own_cost_changes: &[less_if_you_sacrificed_an_artifact_this_turn],
    spell: spell(&[targets::CREATURE], effects::damage_first_target::<4>),
    ..oracle::SUSPICIOUS_DETONATION
};
