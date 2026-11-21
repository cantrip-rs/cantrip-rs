use cantrip_api::*;
use cantrip_mechanics::{effects, firebending, triggers};
use cantrip_oracle as oracle;

/// "Whenever you cast a spell from exile and whenever a permanent you control enters from exile" (CR 603.2, 601.2a, 400.7): two
/// events of one ability. A spell cast from exile (`triggers::you_cast_a_spell_from_exile`), and a permanent that came onto the
/// battlefield from exile under the ability's controller: a flickered creature, not a spell cast from exile, which enters from the
/// stack (CR 608.3a), so a permanent spell cast from exile triggers only once.
fn you_cast_a_spell_from_exile_or_a_permanent_you_control_enters_from_exile(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, from, to, .. } if to.is(ZoneKind::Battlefield) && from.is(ZoneKind::Exile) && q.controller(*new) == me.controller => Fired::PLAIN,
        _ => triggers::you_cast_a_spell_from_exile(q, me, event),
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        firebending::trigger_its_power("Firebending X, where X is Fire Lord Zuko's power. (Whenever this creature attacks, add X {R}. This mana lasts until end of combat.)"),
        triggered(
            when_asked(
                EventMask(EventMask::SPELL_CAST.0 | EventMask::ZONE_CHANGE.0),
                you_cast_a_spell_from_exile_or_a_permanent_you_control_enters_from_exile,
            ),
            effects::plus_one_counter_on_each_creature_you_control,
            "Whenever you cast a spell from exile and whenever a permanent you control enters from exile, put a +1/+1 counter on each creature you control.",
        ),
    ],
    ..oracle::FIRE_LORD_ZUKO
};
