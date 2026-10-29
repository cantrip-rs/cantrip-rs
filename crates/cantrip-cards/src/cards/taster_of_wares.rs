use cantrip_api::*;
use cantrip_mechanics::{choose, permanents_with_subtype_you_control, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Target opponent reveals X cards from their hand, where X is the number of Goblins you control. You choose one of those cards.
/// That player exiles it. If an instant or sorcery card is exiled this way, you may cast it for as long as you control this
/// creature, and mana of any type can be spent to cast that spell" (CR 701.20a, 608.2d, 607.2a): X is counted once, as the ability
/// resolves, among the Goblins its controller controls (this one included, if it is still there); the target opponent chooses
/// which cards to reveal, all of them when the hand holds no more than X; then the controller chooses one, and the opponent exiles
/// it, linked to this creature as it was when the trigger was put on the stack. An empty hand reveals nothing and nothing happens.
fn reveal_and_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(opponent)).to_vec();
    if hand.is_empty() {
        return Ok(());
    }
    let goblin = g.words(item.object).subtype(Subtype::Goblin);
    let count = cantrip_api::count_u32(permanents_with_subtype_you_control(g, item.controller, goblin));
    let revealed = choose::select(g, opponent, item.object, Purpose::Reveal, hand, Selection::new(select::as_many_as_can(count)))?;
    g.reveal(opponent, &revealed);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Exile, revealed, Selection::new(select::exactly(1)))?;
    g.exile_with(&chosen, g.linked(item), false);
    Ok(())
}

/// "If an instant or sorcery card is exiled this way, you may cast it for as long as you control this creature": a continuous
/// effect of the resolved ability (CR 611.2a), so "you" is the ability's controller and the duration ends the first time they
/// stop controlling the creature, for good (CR 611.2b): the card exiled with it (CR 607.2a) can be cast by this creature's
/// controller while no effect has changed its control since the card was exiled. After one has, nobody can cast it (the new
/// controller never was "you", the old one no longer controls it), nor after it leaves the battlefield. Asked whenever the
/// cast would begin (CR 601.3).
fn may_cast_the_exiled_card(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    let exiled = q.history().select(EventQuery::kind(EventMask::ZONE_CHANGE).object(card).to(ZoneKind::Exile)).last();
    player == asker.controller
        && q.exiled_with(q.linked_to(asker)).contains(&card)
        && (q.chars(card).is(CardType::Instant) || q.chars(card).is(CardType::Sorcery))
        && exiled.is_some_and(|e| !q.history().any(EventQuery::kind(EventMask::CONTROL_CHANGED).object(asker.source).after(e.serial)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        reveal_and_exile,
        "When this creature enters, target opponent reveals X cards from their hand, where X is the number of Goblins you control. You choose one of those cards. That player exiles it. If an instant or sorcery card is exiled this way, you may cast it for as long as you control this creature, and mana of any type can be spent to cast that spell.",
    )
    .targets(&[targets::OPPONENT])],
    // "... and mana of any type can be spent to cast that spell" (CR 609.4b).
    exile_casts: &[ExileCast { text: "Cast the exiled instant or sorcery card with mana of any type", allows: may_cast_the_exiled_card, additional: Cost::NONE, any_type: true, lands: false, life_for_mana_value: false }],
    ..oracle::TASTER_OF_WARES
};
