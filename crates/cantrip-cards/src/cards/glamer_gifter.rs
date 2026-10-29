use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Until end of turn, that creature has base power and toughness 4/4 and gains all creature types": two continuous effects
/// until the cleanup step (CR 514.2), 4/4 in layer 7b (CR 613.4b) and the types in layer 4 (CR 613.1d). With no target chosen,
/// or one no longer legal, nothing is changed (CR 608.2b).
fn four_four_every_type(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        g.add_effect(item.object, creature, EffectKind::SetPt(4, 4), Duration::EndOfTurn);
        g.add_effect(item.object, creature, EffectKind::AddAllCreatureTypes, Duration::EndOfTurn);
    });
    Ok(())
}

/// Flash and flying are core keywords.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        four_four_every_type,
        "When this creature enters, choose up to one other target creature. Until end of turn, that creature has base power and toughness 4/4 and gains all creature types.",
    )
    .targets(&[up_to_one(targets::other_creature_filter)])],
    ..oracle::GLAMER_GIFTER
};
