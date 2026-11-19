use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// X as the Enchantment was cast: the number of creatures its trigger may target (CR 107.3m, 603.3d); the permanent keeps the value
/// it was cast with.
fn x_of_this(q: &dyn Query, asker: &Asker) -> u32 {
    q.obj(asker.source).x
}

/// "Exile up to X target creatures. For each of those cards, its owner may play it for as long as it remains exiled" (CR 406.2, 601.3,
/// 305.1): the targets still legal are exiled at once, and the owner of each card (not who controlled the creature) gets a standing
/// permission, for the new object, to cast it or play it as a land, under the usual timing and costs. A token that was exiled has
/// ceased to exist (CR 111.7), is no longer in exile, and gets none.
fn exile_and_owners_may_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exile_all(&targets::legal_objects(g, item));
    for card in exiled {
        let (owner, in_exile) = (g.obj(card).owner, g.obj(card).zone.is(ZoneKind::Exile));
        if in_exile {
            g.allow_play(card, owner, PlayGrant { lands: true, ..PlayGrant::CAST });
        }
    }
    Ok(())
}

/// "Whenever one or more nontoken creatures enter, if one or more of them entered from exile or was cast from exile" (CR 603.2c,
/// 603.4, 603.6a): any player's nontoken creature that came onto the battlefield from exile, or from the stack as a spell cast from
/// exile (the history of the spell that resolved, CR 608.3). The creature rides along as the subject.
fn nontoken_creature_enters_from_exile_or_cast_from_exile(q: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { old, new, from, to, .. } = event else { return None };
    if !to.is(ZoneKind::Battlefield) || q.obj(*new).token || !q.chars(*new).is(CardType::Creature) {
        return None;
    }
    let cast_from_exile = q.history().cast_of(*old).is_some_and(|cast| cast.from == ZoneKind::Exile);
    (from.is(ZoneKind::Exile) || cast_from_exile).then(|| Fired::about(Target::Object(*new))).flatten()
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            exile_and_owners_may_play,
            "When this enchantment enters, exile up to X target creatures. For each of those cards, its owner may play it for as long as it remains exiled.",
        )
        .targets(&[targets::CREATURE.between(0, TargetSpec::COUNTED)])
        .counted_by(x_of_this),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, nontoken_creature_enters_from_exile_or_cast_from_exile),
            |g, item| effects::draw(g, item, 1),
            "Whenever one or more nontoken creatures enter, if one or more of them entered from exile or was cast from exile, you draw a card. This ability triggers only once each turn.",
        )
        .triggers_each_turn(1)
        .once_per_batch(),
    ],
    ..oracle::EXTRAORDINARY_JOURNEY
};
