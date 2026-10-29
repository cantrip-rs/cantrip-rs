use cantrip_api::*;
use cantrip_mechanics::{class, effects, targets, triggers};
use cantrip_oracle as oracle;

/// Level 2: "Whenever a creature you control leaves the battlefield, if it had a counter on it" (CR 603.6c, 603.10a): looked
/// back on, so the creature is judged as it last was on the battlefield - a creature then, under the controller it last had,
/// with whatever counters it had (CR 122.1). Each creature that leaves is its own event and triggers it once, a Class that
/// leaves with them included (CR 603.10a). Only once the Class has reached level 2.
fn creature_with_a_counter_leaves(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, .. }
            if from.is(ZoneKind::Battlefield)
                && class::level(q, me.source) >= 2
                && q.chars(*old).is(CardType::Creature)
                && q.obj(*old).controller == me.controller
                && q.obj(*old).has_counters() =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// Level 3: "Whenever you cast a spell" (CR 601.2i, 603.2), only once the Class has reached level 3.
fn you_cast_a_spell_at_level_three(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if class::level(q, me.source) >= 3 { triggers::you_cast_a_spell(q, me, event) } else { None }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOU_ATTACK, effects::plus_one_counter_on_target, "Whenever you attack, put a +1/+1 counter on target attacking creature.")
            .targets(&[targets::ATTACKING_CREATURE]),
        class::level_two("{2}{W}", "{2}{W}: Level 2"),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, creature_with_a_counter_leaves),
            |g, item| effects::gain_life(g, item, 2),
            "Whenever a creature you control leaves the battlefield, if it had a counter on it, you gain 2 life.",
        ),
        class::level_three("{3}{W}", "{3}{W}: Level 3"),
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_a_spell_at_level_three),
            effects::plus_one_counter_on_each_creature_you_control,
            "Whenever you cast a spell, put a +1/+1 counter on each creature you control.",
        ),
    ],
    ..oracle::LEADERS_TALENT
};
