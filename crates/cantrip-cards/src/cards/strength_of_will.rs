use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature you control gains indestructible and 'Whenever this creature is dealt damage, put that
/// many +1/+1 counters on it.'" (CR 611.2a, 613.1f, 702.12b): two layer 6 effects for the rest of the turn, the keyword and the
/// card's delayed ability 0, a damage trigger of the creature's own. Nothing if the target is illegal as this resolves (CR 608.2b).
fn grant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let this = g.card_id(&CARD);
    g.one_instruction(&mut |g| {
        effects::gain_indestructible_until_end_of_turn(g, item.object, creature);
        g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
        Ok(())
    })
}

/// "Whenever this creature is dealt damage, put that many +1/+1 counters on it." (CR 120.2, 122.1) "That many" is the damage
/// dealt at once, the trigger's matched damage amount; it counts even when the damage is lethal, indestructible having kept the creature. A
/// creature that is no longer on the battlefield gets none.
fn counters_equal_to_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, item.matched_damage_amount() as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], grant),
    // The granted ability (the card's delayed ability 0).
    delayed: &[triggered(
        triggers::THIS_DEALT_DAMAGE,
        counters_equal_to_damage,
        "Whenever this creature is dealt damage, put that many +1/+1 counters on it.",
    )
    .sums_per_subject().damage_amount()],
    ..oracle::STRENGTH_OF_WILL
};
