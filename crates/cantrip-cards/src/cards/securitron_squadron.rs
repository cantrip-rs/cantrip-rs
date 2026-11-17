use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever a creature token you control enters": its own entry too, and its siblings'.
fn token_enters(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield)
                && q.obj(*new).token
                && q.obj(*new).controller == q.controller(source)
                && q.chars(*new).is(CardType::Creature) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    kicker: multikicker(mana("{3}")),
    abilities: &[
        triggers::SQUAD,
        triggered(when(EventMask::ZONE_CHANGE, token_enters), effects::plus_one_counter_on_that_permanent, "Whenever a creature token you control enters, put a +1/+1 counter on it."),
    ],
    ..oracle::SECURITRON_SQUADRON
};
