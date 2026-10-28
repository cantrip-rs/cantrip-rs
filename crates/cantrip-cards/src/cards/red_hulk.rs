use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever Red Hulk is dealt damage, put a +1/+1 counter on him. When you do, he deals damage equal to the number of +1/+1
/// counters on him to any other target." (CR 120.2, 122.1, 603.12): the counter goes on him if he is still the object that was
/// dealt the damage (CR 400.7), and the reflexive ability exists only if it was put on (CR 603.12). Damage dealt by several sources
/// at once triggers it once (`sums_per_subject`).
fn counter_then_reflex(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.is_on_battlefield(item.object) {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

/// "He deals damage equal to the number of +1/+1 counters on him to any other target": counted as the reflexive ability resolves, or
/// as he last existed if he has left the battlefield (CR 113.7a, 608.2h).
fn damage_for_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let counters = g.obj(item.object).counters(CounterKind::PlusOne).max(0) as u16;
    g.damage(item.object, item.target(0), counters);
    Ok(())
}

/// Reach and trample are core keywords.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DEALT_DAMAGE,
        counter_then_reflex,
        "Enrage \u{2014} Whenever Red Hulk is dealt damage, put a +1/+1 counter on him. When you do, he deals damage equal to the number of +1/+1 counters on him to any other target.",
    )
    .sums_per_subject()],
    delayed: &[delayed_trigger(
        damage_for_counters,
        "When you do, he deals damage equal to the number of +1/+1 counters on him to any other target.",
    )
    .targets(&[targets::ANY_OTHER_TARGET])],
    ..oracle::RED_HULK
};
