use cantrip_api::*;
use cantrip_mechanics::history::gained_life_this_turn;
use cantrip_mechanics::{effects, targets, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// "Put two +1/+1 counters on target creature. Infusion -- If you gained life this turn, that creature also gains trample and
/// indestructible until end of turn." The infusion is judged as the spell resolves (CR 608.2c); nothing happens if the target is
/// gone (CR 608.2b).
fn counters_then_infusion(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 2);
    g.end_sentence();
    if gained_life_this_turn(g, item.controller) {
        g.one_instruction(&mut |g| {
            trample_until_end_of_turn(g, item.object, creature);
            effects::gain_indestructible_until_end_of_turn(g, item.object, creature);
            Ok(())
        })?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], counters_then_infusion), ..oracle::EFFLORESCENCE };
