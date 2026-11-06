use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Whenever a creature you control enters": including Jaws of Defeat itself if it enters as a creature (CR 603.6a).
/// The entered creature is the ability's subject, fixed as it triggers.
fn creature_you_control_enters(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && q.controller(*new) == q.controller(source) && q.chars(*new).is(CardType::Creature) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// The targeted opponent loses life equal to the difference between the entered creature's power and toughness, as
/// the ability resolves (CR 608.2h): a creature that has left is read as it last was. No power or toughness reads 0
/// (CR 208.3), and a difference of zero loses no life.
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (Target::Player(opponent), Target::Object(creature)) = (item.target(0), item.subject) else { return Ok(()) };
    let chars = g.chars(creature);
    let difference = (chars.power.unwrap_or(0) as i32 - chars.toughness.unwrap_or(0) as i32).abs();
    if difference > 0 {
        g.lose_life(opponent,(difference) as i32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, creature_you_control_enters), drain, "Whenever a creature you control enters, target opponent loses life equal to the difference between that creature's power and its toughness.")
        .targets(&[targets::OPPONENT])],
    ..oracle::JAWS_OF_DEFEAT
};
