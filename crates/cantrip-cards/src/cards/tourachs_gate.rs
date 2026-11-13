use cantrip_api::*;
use cantrip_mechanics::{attach, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Sacrifice a Thrull: Put three time counters on this Aura." (CR 122.1).
fn put_three_time_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Time, 3);
    Ok(())
}

/// "Sacrifice a Thrull" (CR 118.1), read through the words of this Aura's text (CR 612).
fn thrull(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Thrull))
}

/// "At the beginning of your upkeep, remove a time counter from this Aura. If there are no time counters on this Aura,
/// sacrifice it." (CR 122.1, 608.2c, 701.21a).
fn remove_time_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.remove_counters(item.object, CounterKind::Time, 1);
    g.end_sentence();
    if g.obj(item.object).counters(CounterKind::Time) == 0 {
        effects::sacrifice_self(g, item)?;
    }
    Ok(())
}

/// "Tap enchanted land: Attacking creatures you control get +2/-1 until end of turn." (CR 611.2c).
fn attacking_creatures_get_plus_two_minus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    effects::pump_creatures_where(g, item.object, |q, id| q.is_attacking(id) && q.controller(id) == you, 2, -1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(sacrifice(1, thrull), put_three_time_counters, "Sacrifice a Thrull: Put three time counters on this Aura."),
        triggered(
            triggers::YOUR_UPKEEP,
            remove_time_counter,
            "At the beginning of your upkeep, remove a time counter from this Aura. If there are no time counters on this Aura, sacrifice it.",
        ),
        ability(
            tap_untapped(1, attach::attached_to_it),
            attacking_creatures_get_plus_two_minus_one,
            "Tap enchanted land: Attacking creatures you control get +2/-1 until end of turn. Activate only if enchanted land is untapped.",
        )
        .condition(attach::enchanted_permanent_untapped),
    ],
    enchant: Some(targets::LAND_YOU_CONTROL),
    ..oracle::TOURACHS_GATE
};
