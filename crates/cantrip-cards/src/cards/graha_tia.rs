use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever one or more other creatures and/or artifacts you control die" (CR 700.4, 603.2c, 603.10a): a permanent other than
/// G'raha Tia that was a creature or an artifact as it last was on the battlefield, under the controller it last had. The ability
/// is `once_per_batch`, so those that die together (G'raha Tia among them) trigger it once.
fn creatures_or_artifacts_die(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if *old != me.source
                && from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.obj(*old).controller == me.controller
                && (q.chars(*old).is(CardType::Creature) || q.chars(*old).is(CardType::Artifact)) =>
        {
            Fired::about(Target::Object(*old))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, creatures_or_artifacts_die),
        |g, item| effects::draw(g, item, 1),
        "The Allagan Eye \u{2014} Whenever one or more other creatures and/or artifacts you control die, draw a card. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)
    .once_per_batch()],
    ..oracle::GRAHA_TIA
};
