use cantrip_api::*;
use cantrip_mechanics::{effects, targets, teamwork};
use cantrip_oracle as oracle;

/// "Target creature gains double strike until end of turn. If this spell was cast using teamwork, that creature also gains trample
/// until end of turn." (CR 702.194b, 611.2a, 702.4, 702.19): two sentences, the second only with teamwork; a copy of a spell cast
/// using teamwork was cast that way too (CR 707.10). Nothing if the target is no longer legal (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::DoubleStrike);
    g.end_sentence();
    if teamwork::used(item) {
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Trample);
    }
    Ok(())
}

/// Teamwork 1 (as an additional cost to cast this spell, you may tap any number of creatures you control with total power 1 or
/// more): an optional cost announced before the target, which is chosen after it.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    late_targets: true,
    optional_cost: Some(teamwork::optional(1)),
    labels: &["Cast without teamwork", "Teamwork 1: Tap any number of creatures you control with total power 1 or more"],
    ..oracle::TEAM_TACTICS
};
