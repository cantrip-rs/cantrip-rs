use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, target artifact or creature becomes an artifact creature with base power and toughness 4/5. Draw a
/// card." It becomes an "artifact creature", so it keeps all of its prior card types and subtypes (CR 205.1b): a land
/// stays a land with its land types. The base power and toughness are set in layer 7b (CR 613.4b), so
/// counters and pumps count on top, and a later setting effect overrides it by timestamp. With its one target illegal the
/// spell does not resolve (CR 608.2b), so no card is drawn.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::first_target_becomes_artifact_creature_with_base_pt(g, item, 4, 5) {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE], effect),
    ..oracle::MIND_TRANSFER_PROTOCOL
};
