use cantrip_api::*;
use cantrip_mechanics::library::exile_top_card_play_until_end_of_next_turn;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "A Goblin, Orc, or Army": the object has one of the three creature types as the layers make it (as it last existed, for one that
/// has left), each read through the words of the text of the object whose ability asks (CR 612).
fn is_a_goblin_orc_or_army(q: &dyn Query, asker_source: ObjectId, id: ObjectId) -> bool {
    let words = q.words(asker_source);
    let chars = q.chars(id);
    chars.has_subtype(words.subtype(Subtype::Goblin)) || chars.has_subtype(words.subtype(Subtype::Orc)) || chars.has_subtype(words.subtype(Subtype::Army))
}

/// "Whenever you put one or more counters on a Goblin, Orc, or Army you control" (CR 122.6a, 603.2c): counters put by this ability's
/// controller on a permanent they control that is one of the three, of any kind. Each such permanent is its own event (counters put
/// on several at once trigger it for each), and the counters of several kinds put on one at once are one trigger for it: the
/// batch is split by the permanent.
fn you_put_counters_on_a_goblin_orc_or_army_you_control(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, amount, by, .. }
            if *amount > 0
                && *by == me.controller
                && q.is_on_battlefield(*object)
                && q.controller(*object) == me.controller
                && is_a_goblin_orc_or_army(q, me.source, *object) =>
        {
            Some(Fired { subject: Target::Object(*object), data: [0, object.0] })
        }
        _ => None,
    }
}

/// "Whenever another Goblin, Orc, or Army you control dies" (CR 700.4, 603.10a), looked back on: a creature other than this one that
/// was one of the three, under this ability's controller's control, as it last was on the battlefield (a Great Goblin dying with it
/// sees it die).
fn another_goblin_orc_or_army_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if *old != me.source
                && from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.chars(*old).is(CardType::Creature)
                && q.obj(*old).controller == me.controller
                && is_a_goblin_orc_or_army(q, me.source, *old) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "The Great Goblin deals 2 damage to target opponent" (CR 120.3): if the target is still legal.
fn deal_two_damage_to_target_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::COUNTERS, you_put_counters_on_a_goblin_orc_or_army_you_control),
            deal_two_damage_to_target_opponent,
            "Whenever you put one or more counters on a Goblin, Orc, or Army you control, The Great Goblin deals 2 damage to target opponent.",
        )
        .once_per_batch()
        .batch_by_data()
        .targets(&[targets::OPPONENT]),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, another_goblin_orc_or_army_you_control_dies),
            exile_top_card_play_until_end_of_next_turn,
            "Whenever another Goblin, Orc, or Army you control dies, exile the top card of your library. You may play it until the end of your next turn.",
        ),
    ],
    ..oracle::THE_GREAT_GOBLIN
};
