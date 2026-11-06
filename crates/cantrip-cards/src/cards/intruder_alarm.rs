use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever a creature enters": any creature under any player's control, and the Alarm itself if it enters as one.
fn creature_enters(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Battlefield) && q.chars(*new).is(CardType::Creature) => Fired::PLAIN,
        _ => None,
    }
}

/// Untap all creatures: every creature on the battlefield, all players'.
fn untap_all(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> =
        g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    g.untap_together(&creatures);
    Ok(())
}

/// "Creatures don't untap during their controllers' untap steps" (CR 502.3): every creature, whoever controls it, as the
/// untap step begins. A rule of the Alarm, never an ability of the creatures (CR 113.1a), so a creature that has lost its
/// abilities is still under it (CR 613.1f).
fn creatures_dont_untap(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, creature_enters), untap_all, "Whenever a creature enters, untap all creatures.")],
    untap_bans: &[creatures_dont_untap],
    ..oracle::INTRUDER_ALARM
};
