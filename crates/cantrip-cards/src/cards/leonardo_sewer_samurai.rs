use cantrip_api::*;
use cantrip_mechanics::sneak;
use cantrip_oracle as oracle;

/// "During your turn, you may cast creature spells with power or toughness 1 or less from your graveyard": a way a permanent
/// gives (CR 604, 601.3), a permission for the card's own mana cost (`GrantedWay::permission`), so an alternative cost may be
/// paid instead. The power and toughness are the card's in the graveyard (CR 208.3, 109.3: a card in a graveyard has its
/// own characteristics), read as `chars` gives them there. It is not a limit of once, so the history is not asked.
fn cast_small_creature(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    if player != asker.controller || q.state().turn.active != player {
        return None;
    }
    let c = q.chars(card);
    if !c.is(CardType::Creature) || !(c.power.is_some_and(|p| p <= 1) || c.toughness.is_some_and(|t| t <= 1)) {
        return None;
    }
    Some(Cost::mana(q.card(c.card).cost))
}

/// "If you cast a spell this way": a spell whose cast the history records as made by this permanent's permission (`Cast::by`),
/// the cast of the spell it is or of the spell it became (CR 400.7, 608.3).
fn cast_by_this(q: &dyn Query, source: ObjectId, object: ObjectId) -> bool {
    let cast_by = q.history().all().iter().rev().find_map(|r| match &r.event {
        Event::Cast(c) if c.spell == object => Some(c.granted_by().is_some_and(|g| g.object == source)),
        _ => None,
    });
    cast_by == Some(true)
}

/// "That creature enters with a finality counter on it" (CR 614.1c, 122.6): a permanent entering from the stack that was
/// cast by this permanent's permission.
fn a_creature_cast_this_way_enters(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.to.is(ZoneKind::Battlefield) && z.from.is(ZoneKind::Stack) && cast_by_this(q, asker.source, z.object))
}

fn add_a_finality_counter(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.counters[CounterKind::Finality as usize] += 1;
    }
}

pub static CARD: CardDef = CardDef {
    ways: &[sneak::way(mana("{2}{W}{W}"), "Sneak {2}{W}{W}")],
    granted_ways: &[GrantedWay {
        text: "During your turn, you may cast creature spells with power or toughness 1 or less from your graveyard.",
        cost: cast_small_creature,
        permission: true,
        ..GrantedWay::CAST
    }],
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        a_creature_cast_this_way_enters,
        add_a_finality_counter,
        "enters with a finality counter on it",
    )],
    ..oracle::LEONARDO_SEWER_SAMURAI
};
