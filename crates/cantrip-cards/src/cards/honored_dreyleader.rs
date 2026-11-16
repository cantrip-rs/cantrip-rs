use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// Whether `id` is a Squirrel or a Food, the types read through the words of `source`'s text (CR 612).
fn squirrel_or_food(q: &dyn Query, source: ObjectId, id: ObjectId) -> bool {
    let words = q.words(source);
    let chars = q.chars(id);
    chars.has_subtype(words.subtype(Subtype::Squirrel)) || chars.has_subtype(words.subtype(Subtype::Food))
}

/// "Whenever another Squirrel or Food you control enters" (CR 603.6a): a permanent other than this one, under your control as
/// it entered, that is a Squirrel or a Food (a token too).
fn another_squirrel_or_food_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if *new != me.source && to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller && squirrel_or_food(q, me.source, *new) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Put a +1/+1 counter on it for each other Squirrel and/or Food you control" (CR 122.1, 608.2h): counted as it resolves, each permanent once,
/// whether it is a Squirrel, a Food or both.
fn plus_one_counter_for_each_other_squirrel_or_food_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let many = g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && squirrel_or_food(g, item.object, id)).count();
    g.add_counters(item.object, CounterKind::PlusOne, many as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            plus_one_counter_for_each_other_squirrel_or_food_you_control,
            "When this creature enters, put a +1/+1 counter on it for each other Squirrel and/or Food you control.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, another_squirrel_or_food_you_control_enters),
            effects::plus_one_counter_on_this,
            "Whenever another Squirrel or Food you control enters, put a +1/+1 counter on this creature.",
        ),
    ],
    ..oracle::HONORED_DREYLEADER
};
