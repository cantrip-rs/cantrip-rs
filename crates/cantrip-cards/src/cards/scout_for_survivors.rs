use cantrip_api::*;
use cantrip_mechanics::{select, targets};
use cantrip_oracle as oracle;

/// "Return up to three target creature cards with total mana value 3 or less from your graveyard to the battlefield." It may be
/// cast with no target (CR 601.2c). The total is judged as the targets are chosen and again on resolution: if it has grown past 3
/// every target is illegal (CR 608.2b); otherwise the targets still legal enter under your control (CR 110.2a). "Put a +1/+1
/// counter on each of them." (CR 122.1): on the cards that entered, a new object each (CR 400.7), after the first sentence is
/// done (CR 608.2c), all of them at once, one instruction (CR 122.6) - and only those: a card a replacement sent elsewhere
/// (Containment Priest's exile) never became one of them.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let cards: Vec<(ObjectId, PlayerId)> = targets::legal_objects(g, item).into_iter().map(|c| (c, you)).collect();
    let entered = g.put_all_onto_battlefield(&cards)?;
    g.end_sentence();
    let counters: Vec<(ObjectId, CounterKind, i64)> = entered.into_iter().filter(|&id| g.is_on_battlefield(id)).map(|id| (id, CounterKind::PlusOne, 1)).collect();
    g.add_counters_together(&counters);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[TargetSpec { min: 0, max: 3, ..targets::CREATURE_CARD_IN_YOUR_GRAVEYARD }
            .together(select::total_mana_value_at_most(3))
            .labelled("Creature cards from your graveyard to return to the battlefield")],
        effect,
    ),
    ..oracle::SCOUT_FOR_SURVIVORS
};
