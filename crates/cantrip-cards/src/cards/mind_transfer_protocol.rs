use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Until end of turn, target artifact or creature becomes an artifact creature with base power and toughness 4/5. Draw a
/// card." It is exactly an artifact creature (CR 205.1a): an enchantment-creature or a land creature stops being the other
/// type; its supertypes, colors and other subtypes stay. The base power and toughness are set in layer 7b (CR 613.4b), so
/// counters and pumps count on top, and a later setting effect overrides it by timestamp. With its one target illegal the
/// spell does not resolve (CR 608.2b), so no card is drawn.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        g.add_effect(item.object, permanent, EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact, CardType::Creature])), Duration::EndOfTurn);
        g.add_effect(item.object, permanent, EffectKind::SetPt(4, 5), Duration::EndOfTurn);
    });
    g.end_sentence();
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE], effect),
    ..oracle::MIND_TRANSFER_PROTOCOL
};
