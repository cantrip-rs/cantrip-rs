use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever a non-Human creature enters" (CR 603.2): any creature entering the battlefield that lacks the Human subtype
/// - including an opponent's, and Uhura herself is a Human so she never triggers it. Artifacts that are not creatures
/// and Humans entering do not trigger it.
fn non_human_creature_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && {
                let id = *new;
                q.chars(id).is(CardType::Creature) && !q.chars(id).has_subtype(q.words(me.source).subtype(Subtype::Human))
            } =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, non_human_creature_enters),
        |g, item| effects::may_pay_to_draw(g, item, cost("{1}")),
        "Whenever a non-Human creature enters, you may pay {1}. If you do, draw a card.",
    )],
    ..oracle::UHURA_XENOLINGUIST
};
