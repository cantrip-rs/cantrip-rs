use cantrip_api::*;
use cantrip_mechanics::effects::{gain_indestructible_until_end_of_turn, gain_lifelink_until_end_of_turn};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature. It gains lifelink and indestructible until end of turn." (CR 122.1, 611.2a): the
/// counter, then in a second sentence (CR 608.2c) two layer-6 effects of this spell on the same creature, one instruction. Nothing
/// at all if the creature is no longer a legal target as the spell resolves (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    g.end_sentence();
    instruction(g, |g| {
        gain_lifelink_until_end_of_turn(g, item.object, creature);
        gain_indestructible_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::TAKE_UP_THE_SHIELD };
