use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on up to one target creature. Target player gains 2 life." (CR 122.1a, 119.3): each target is judged
/// on its own as the spell resolves (CR 608.2b), so the life is gained when the creature is gone, and the counter put when
/// the player is. A spell with every target illegal does not resolve at all, the core's.
fn counter_and_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    g.end_sentence();
    if let Some(player) = targets::player(g, item, 1) {
        g.gain_life(player, 2);
    }
    Ok(())
}

/// The Adventure (CR 715): Meager Meal.
pub static MEAGER_MEAL: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.optional(), targets::PLAYER], counter_and_life),
    ..oracle::MEAGER_MEAL
};

pub static CARD: CardDef = CardDef { adventure: Some(&MEAGER_MEAL), ..oracle::GOLLUM_SILENT_SLINKER };

pub static EXTRA: &[&CardDef] = &[&MEAGER_MEAL];
