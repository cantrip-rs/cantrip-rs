use cantrip_api::*;
use cantrip_mechanics as mechanics;
use cantrip_oracle as oracle;

/// "Whenever Captain Janeway or another creature you control enters, that creature explores." The source entering is
/// Captain Janeway; otherwise the entering permanent must be a creature controlled by her controller. Carry the entering
/// creature as the event's subject, not a target (CR 603.2, 603.6a).
fn janeway_or_your_creature_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    if !to.is(ZoneKind::Battlefield) {
        return None;
    }
    if *new == me.source
        || (q.controller(*new) == me.controller && q.chars(*new).is(CardType::Creature))
    {
        Fired::about(Target::Object(*new))
    } else {
        None
    }
}

fn explore_that_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject { mechanics::explore(g, creature)?; }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, janeway_or_your_creature_enters),
        explore_that_creature,
        "Whenever Captain Janeway or another creature you control enters, that creature explores.",
    )],
    // "You may play an additional land on each of your turns" (CR 305.2).
    play_rules: &[PlayRule::ExtraLandPlays(1)],
    ..oracle::CAPTAIN_KATHRYN_JANEWAY
};
