use cantrip_api::*;
use cantrip_mechanics::hand::discard_chosen_nonland_from_revealed_hand;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card. Put two +1/+1 counters on
/// up to one target creature." Each target is judged as the spell resolves (CR 608.2b): with the opponent gone the counters are
/// still put on, and with no creature chosen or still legal the hand is still taken from. The hand is looked at even when it holds
/// no nonland card to take.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Some(opponent) = item.target(0).player()
    {
        discard_chosen_nonland_from_revealed_hand(g, item, opponent)?;
    }
    g.end_sentence();
    let Some(creature) = targets::object(g, item, 1) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT, targets::CREATURE.optional()], effect),
    ..oracle::RENDER_SPEECHLESS
};
