use cantrip_api::*;
use cantrip_mechanics::{effects, flashback, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature you control. It gains vigilance until end of turn." Two sentences, so the
/// counter and the keyword are separate instructions (CR 608.2c); nothing if the target is no longer legal (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    g.end_sentence();
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Vigilance);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], effect),
    ways: &[flashback::way(cost("{W}"), "Flashback {W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)")],
    ..oracle::DIG_SITE_INVENTORY
};
