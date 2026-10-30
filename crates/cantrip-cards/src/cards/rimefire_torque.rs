use cantrip_api::*;
use cantrip_mechanics::{chosen_type, copy, delayed, triggers};
use cantrip_oracle as oracle;

/// "Whenever a permanent you control of the chosen type enters" (CR 603.6a): a permanent of any card type, this one
/// included, under your control as it entered, that is the creature type chosen for the ability that asks
/// (`Query::linked_to`), as the layers make it as it entered.
fn permanent_of_chosen_type_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller => {
            let chosen = q.state().chosen_subtype(q.linked_to(me))?;
            q.chars(*new).has_subtype(chosen).then_some(Fired::default())
        }
        _ => None,
    }
}

fn charge(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Charge, 1);
    Ok(())
}

/// "{T}, Remove three charge counters: When you next cast an instant or sorcery spell this turn, copy it": the ability
/// sets up a delayed triggered ability, number 0 of the card's `delayed` list, that triggers once on the next such spell
/// its controller casts this turn and ends with the turn if none is cast (CR 603.7b).
fn watch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::next_this_turn(g, item, 0, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(chosen_type::enters_choosing_a_creature_type),
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, permanent_of_chosen_type_enters),
            charge,
            "Whenever a permanent you control of the chosen type enters, put a charge counter on this artifact.",
        ),
        ability(
            TAP.and(remove_counters(CounterKind::Charge, 3)),
            watch,
            "{T}, Remove three charge counters from this artifact: When you next cast an instant or sorcery spell this turn, copy it. You may choose new targets for the copy.",
        ),
    ],
    delayed: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::you_cast_instant_or_sorcery),
        copy::copy_the_cast_spell,
        "When you next cast an instant or sorcery spell this turn, copy it. You may choose new targets for the copy.",
    )],
    ..oracle::RIMEFIRE_TORQUE
};
