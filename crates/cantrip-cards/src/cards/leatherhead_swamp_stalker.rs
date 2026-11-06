use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("RemoveACounter", "remove a counter from her?", "When you do, destroy target artifact or enchantment that player controls", "Remove a counter", "Keep it");

/// "A counter" names no kind (CR 118.3), so the choice is made for her controller in their favour: drawbacks first, then
/// counters with no rules meaning, then the ones whose loss costs something, indestructible last of all.
const SHED_ORDER: [CounterKind; 13] = [
    CounterKind::MinusOne,
    CounterKind::Stun,
    CounterKind::Finality,
    CounterKind::Defense,
    CounterKind::Corpse,
    CounterKind::Charge,
    CounterKind::Lore,
    CounterKind::Loyalty,
    CounterKind::Flying,
    CounterKind::Hexproof,
    CounterKind::Lifelink,
    CounterKind::PlusOne,
    CounterKind::Indestructible,
];

fn counter_to_shed(q: &dyn Query, permanent: ObjectId) -> Option<CounterKind> {
    let o = q.obj(permanent);
    SHED_ORDER.into_iter().find(|&k| o.counters(k) > 0).or_else(|| CounterKind::ALL.into_iter().find(|&k| o.counters(k) > 0))
}

/// "You may remove a counter from her. When you do, destroy target artifact or enchantment that player controls." Not
/// offered when she has no counter. The reflexive trigger (CR 603.12) is queued once the counter is off.
fn remove_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.object;
    if !targets::permanent(g, me) {
        return Ok(());
    }
    let Some(kind) = counter_to_shed(g, me) else { return Ok(()) };
    if !cantrip_mechanics::effects::ask_may(g, item.controller, me, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    g.remove_counters(me, kind, 1);
    let this = g.card_id(&CARD);
    g.reflexive_trigger(me, item.controller, AbilityRef::delayed(this, 0), item.subject);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "Leatherhead enters with a hexproof counter on her" (CR 122.1b, 614.1c).
    enters: Some(cantrip_mechanics::effects::enters_with_a_counter::<{ CounterKind::Hexproof as usize }>),
    abilities: &[triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT, remove_counter, "Whenever Leatherhead deals combat damage to a player, you may remove a counter from her. When you do, destroy target artifact or enchantment that player controls.")],
    delayed: &[delayed_trigger(cantrip_mechanics::effects::destroy_first_target, "When you do, destroy target artifact or enchantment that player controls.")
        .targets(&[target(targets::artifact_or_enchantment_that_player_controls)])],
    ..oracle::LEATHERHEAD_SWAMP_STALKER
};
