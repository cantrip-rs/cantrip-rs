use cantrip_api::*;
use cantrip_mechanics::effects::{ask_may, enters_with_minus_one_counters, return_first_target_to_battlefield};
use cantrip_mechanics::{targets, triggers::YOUR_END_STEP};
use cantrip_oracle as oracle;

/// "Creature card with power 2 or less from your graveyard" (CR 115.1, 208.1): its power as a card in the graveyard has it.
fn creature_card_with_power_2_or_less_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Creature])
        && matches!(t, Target::Object(id) if q.chars(id).power.is_some_and(|p| p <= 2))
}

const CREATURE_CARD_WITH_POWER_2_OR_LESS: TargetSpec = target(creature_card_with_power_2_or_less_in_your_graveyard);

/// "You may remove a counter from this creature. When you do, ..." (CR 603.12): the reflexive ability triggers only if a
/// counter was removed, and its target is chosen as it is put on the stack; with no such card it is not put on the stack.
fn end_step(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !ask_may(g, item.controller, item.object, Purpose::Optional)? {
        return Ok(());
    }
    if g.remove_a_counter_from_among(item.controller, item.object, &[item.object])? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_minus_one_counters::<2>),
    abilities: &[triggered(
        YOUR_END_STEP,
        end_step,
        "At the beginning of your end step, you may remove a counter from this creature. When you do, return target creature card with power 2 or less from your graveyard to the battlefield.",
    )],
    delayed: &[delayed_trigger(return_first_target_to_battlefield, "When you do, return target creature card with power 2 or less from your graveyard to the battlefield.")
        .targets(&[CREATURE_CARD_WITH_POWER_2_OR_LESS])],
    ..oracle::SLUMBERING_WALKER
};
