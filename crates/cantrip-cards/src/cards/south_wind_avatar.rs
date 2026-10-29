use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::YOU_GAIN_LIFE;
use cantrip_oracle as oracle;

/// "Whenever another creature you control dies" (CR 700.4, 603.10a), looked back on: a creature other than this one, under
/// the controller it last had, once for each. It is the subject of the trigger, so "its toughness" can be read as it last was.
fn another_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if *old != me.source
                && from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.chars(*old).is(CardType::Creature)
                && q.obj(*old).controller == me.controller =>
        {
            Fired::about(Target::Object(*old))
        }
        _ => None,
    }
}

/// "You gain life equal to its toughness": the toughness the creature last had on the battlefield (the ruling, CR 608.2h),
/// or none to gain if it was not above zero.
fn gain_its_toughness(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let toughness = g.chars(creature).toughness.unwrap_or_default();
    effects::gain_life(g, item, i32::from(toughness).max(0))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when_asked(EventMask::ZONE_CHANGE, another_creature_you_control_dies), gain_its_toughness, "Whenever another creature you control dies, you gain life equal to its toughness."),
        triggered(YOU_GAIN_LIFE, effects::each_opponent_loses_1_life, "Whenever you gain life, each opponent loses 1 life."),
    ],
    ..oracle::SOUTH_WIND_AVATAR
};
