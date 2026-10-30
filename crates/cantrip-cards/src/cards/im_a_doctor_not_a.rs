use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature you control. Untap it. Until end of turn, it gets +1/+1 and becomes a
/// Doctor." The counter and the untap happen first; the +1/+1 and the creature type are one continuous effect until
/// end of turn (CR 611.2a), and "becomes a Doctor" replaces every other creature type (CR 205.1a). An illegal target
/// does nothing (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let words = g.words(item.object);
    g.add_counters(creature, CounterKind::PlusOne, 1);
    g.end_sentence();
    g.untap(creature);
    instruction(g, |g| {
        pump(g, item.object, creature, 1, 1);
        g.add_effect(item.object, creature, EffectKind::SetCreatureType(words.subtype(Subtype::Doctor)), Duration::EndOfTurn);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], effect),
    ..oracle::IM_A_DOCTOR_NOT_A
};
