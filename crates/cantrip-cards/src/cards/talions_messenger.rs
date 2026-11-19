use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets, triggers};
use cantrip_oracle as oracle;

/// "Draw a card, then discard a card. When you discard a card this way, put a +1/+1 counter on target Faerie you control."
/// (CR 701.9, 603.12): the reflexive ability is created only when a card was discarded (not for an empty hand), whatever card it
/// was. Its Faerie is chosen as it goes on the stack, so it can be the creature that has just attacked.
fn loot_then_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::loot_discarding(g, item.controller, item.object)?.is_empty() {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACKS, triggers::you_attack_with_a_creature_of_subtype::<{ lands::sub(Subtype::Faerie) }>),
        loot_then_counter,
        "Whenever you attack with one or more Faeries, draw a card, then discard a card. When you discard a card this way, put a +1/+1 counter on target Faerie you control.",
    )
    .once_per_batch()],
    delayed: &[delayed_trigger(effects::plus_one_counter_on_target, "When you discard a card this way, put a +1/+1 counter on target Faerie you control.")
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Faerie) }>)])],
    ..oracle::TALIONS_MESSENGER
};
