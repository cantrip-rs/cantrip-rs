use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Return target creature card from your graveyard to the battlefield. Put a +2/+2 counter on that creature if the exiled
/// creature was a Thrull." (CR 608.2c, 122.1). The creature exiled to pay the additional cost is asked of the history, as
/// it last existed (CR 608.2h): a Thrull token or an animated land that has since left is still known for what it was.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let returned = g.put_onto_battlefield(card, item.controller, false)?;
    g.end_sentence();
    if exiled_a_thrull(g, item) && g.is_on_battlefield(returned) {
        g.add_counters(returned, CounterKind::Plus2Plus2, 1);
    }
    Ok(())
}

/// "If the exiled creature was a Thrull": the creature exiled for "As an additional cost to cast this spell, exile a
/// creature you control" (CR 118.8), read through the words of this spell's text (CR 612).
fn exiled_a_thrull(q: &dyn Query, item: &StackItem) -> bool {
    let thrull = q.words(item.object).subtype(Subtype::Thrull);
    q.history()
        .spell_cost_objects(item.object, CostOrigin::PrintedAdditional, 0, CostPart::Permanents)
        .any(|id| q.chars(id).is(CardType::Creature) && q.chars(id).has_subtype(thrull))
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], effect),
    additional_cost: Some(exile_permanents(1, targets::filters::creature)),
    ..oracle::SOUL_EXCHANGE
};
