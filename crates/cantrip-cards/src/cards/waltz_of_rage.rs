use cantrip_api::*;
use cantrip_mechanics::{fight, library, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature you control deals damage equal to its power to each other creature. Until end of turn, whenever a creature you
/// control dies, exile the top card of your library. You may play it until the end of your next turn." The creature is the source
/// of the damage, not the spell (the ruling), read as the spell resolves (CR 608.2h). Then the delayed triggered ability is made
/// (CR 603.7b), so it sees the creatures that die to this very damage, yours among them; with the target illegal as it resolves
/// nothing happens, the delayed ability included (CR 608.2b).
fn deal_damage_then_watch_for_deaths(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    fight::deals_damage_equal_to_its_power_to_each_other_creature(g, creature);
    let this = g.card_id(&CARD);
    g.delayed_trigger(DelayedWhen::ThisTurn, item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], deal_damage_then_watch_for_deaths),
    delayed: &[triggered(
        triggers::CREATURE_YOU_CONTROL_DIES,
        library::exile_top_card_play_until_end_of_next_turn,
        "Whenever a creature you control dies, exile the top card of your library. You may play it until the end of your next turn.",
    )],
    ..oracle::WALTZ_OF_RAGE
};
