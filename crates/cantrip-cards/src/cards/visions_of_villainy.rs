use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast if you control a Villain" (CR 601.2f): judged once, as the total cost is determined, and
/// generic mana only (CR 118.7a); the Villain type is read through the words of this spell's text (CR 612).
fn less_with_a_villain(q: &dyn Query, spell: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let villain = q
        .state()
        .battlefield()
        .iter()
        .any(|&id| q.controller(id) == player && q.chars(id).has_subtype(q.words(spell).subtype(Subtype::Villain)));
    if villain { CostDelta::less(1) } else { CostDelta::NONE }
}

/// "You draw two cards and lose 2 life." (CR 121.1, 118.4): both happen to the spell's controller, in that order.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    g.lose_life(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), own_cost_changes: &[less_with_a_villain], ..oracle::VISIONS_OF_VILLAINY };
