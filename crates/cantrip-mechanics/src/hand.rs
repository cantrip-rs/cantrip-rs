//! Looking into a player's hand and taking a card from it (CR 701.9, 701.17).

use crate::choose;
use cantrip_api::*;

/// "Target player reveals their hand. You choose a card from it [that passes `eligible`]": the hand is revealed, and the
/// ability's controller chooses one eligible card. The revealing is the looking; when nothing in the hand is eligible
/// there is no choice to make, so none is asked (CR 608.2d), and nothing at all for an empty hand. The cards chosen are left for the
/// caller to discard or exile. Duress, Thoughtseize and Inquisition of Kozilek print it (`purpose` is the question's: a
/// discard (`Purpose::TakeFromHand`) or an exile (`Purpose::Exile`): the question says which, to whoever is choosing).
pub fn choose_from_revealed_hand(
    g: &mut dyn Game,
    item: &StackItem,
    player: PlayerId,
    purpose: Purpose,
    eligible: fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    choose_from_revealed_hand_of_kind(g, item, player, purpose, "", eligible)
}

/// `choose_from_revealed_hand` for a card that says what it may take ("nonland", "artifact or creature"): `kind` is its own
/// words, which the question says to the chooser. Intimidation Tactics, Thoughtseize and Grief print one.
pub fn choose_from_revealed_hand_of_kind(
    g: &mut dyn Game,
    item: &StackItem,
    player: PlayerId,
    purpose: Purpose,
    kind: &'static str,
    eligible: fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    if hand.is_empty() {
        return Ok(Vec::new());
    }
    g.reveal(player, &hand);
    let eligible: Vec<ObjectId> = hand.iter().copied().filter(|&c| eligible(&*g, c)).collect();
    Ok(choose::cards_from_kind(g, item.controller, item.object, purpose, kind, hand, eligible, 1, false)?.into_iter().take(1).collect())
}

/// `choose_from_revealed_hand`, then the player discards what was chosen, together (CR 701.9a), still revealed (CR
/// 701.20a, `Game::discard_revealed`). The cards discarded.
pub fn discard_chosen_from_revealed_hand(
    g: &mut dyn Game,
    item: &StackItem,
    player: PlayerId,
    purpose: Purpose,
    eligible: fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    discard_chosen_from_revealed_hand_of_kind(g, item, player, purpose, "", eligible)
}

/// `discard_chosen_from_revealed_hand` for a card that says what may be taken ("nonland"): Thoughtseize, Grief and Humiliate.
pub fn discard_chosen_from_revealed_hand_of_kind(
    g: &mut dyn Game,
    item: &StackItem,
    player: PlayerId,
    purpose: Purpose,
    kind: &'static str,
    eligible: fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let chosen = choose_from_revealed_hand_of_kind(g, item, player, purpose, kind, eligible)?;
    g.discard_revealed(&chosen);
    Ok(chosen)
}

/// "[Target opponent] reveals their hand. You choose a nonland card from it. That player discards that card." (CR 701.9a): the
/// hand is revealed, and when no nonland card is there to take there is no choice to make, so none is asked (CR 608.2d), as for any
/// `choose_from_revealed_hand`. Grief, Humiliate and
/// Down, Down to Goblin-town print it.
pub fn discard_chosen_nonland_from_revealed_hand(g: &mut dyn Game, item: &StackItem, player: PlayerId) -> Result<Vec<ObjectId>, Suspend> {
    discard_chosen_from_revealed_hand_of_kind(g, item, player, Purpose::TakeFromHand, "nonland", |q, c| !q.chars(c).is(CardType::Land))
}

/// "[Target opponent] reveals their hand. You choose a nonland permanent card from it. That player discards that card." (CR 701.9a,
/// 110.4a): only an artifact, battle, creature, enchantment or planeswalker card may be taken, and none being there, no choice is
/// asked (CR 608.2d). Solve for Disappointment and Auntie's Sentence print it.
pub fn discard_chosen_nonland_permanent_from_revealed_hand(
    g: &mut dyn Game,
    item: &StackItem,
    player: PlayerId,
) -> Result<Vec<ObjectId>, Suspend> {
    discard_chosen_from_revealed_hand_of_kind(g, item, player, Purpose::TakeFromHand, "nonland permanent", |q, card| {
        crate::targets::is_nonland_permanent_card(&q.chars(card))
    })
}

/// "Look at target player's hand. You may choose a card from it [that passes `eligible`]": the ability's controller sees the hand
/// (it is shown to them alone, not revealed) and may choose one eligible card. Looking is part of the ability, so the choice is
/// asked of any hand that is not empty, even when nothing in it is eligible; an empty hand is nothing to look at. The card chosen
/// is left for the caller to exile or discard; `purpose` and `kind` word the question. Deep Cavern Bat, Invasion of Gobakhan and
/// Oildeep Gearhulk print it.
pub fn look_and_may_choose(
    g: &mut dyn Game,
    item: &StackItem,
    player: PlayerId,
    purpose: Purpose,
    kind: &'static str,
    eligible: fn(&dyn Query, ObjectId) -> bool,
) -> Result<Option<ObjectId>, Suspend> {
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    if hand.is_empty() {
        return Ok(None);
    }
    let eligible: Vec<ObjectId> = hand.iter().copied().filter(|&c| eligible(&*g, c)).collect();
    let rule = Selection::listed(Count::UpTo(u32::from(!eligible.is_empty())), &hand, eligible).called(kind);
    Ok(crate::choose::select(g, item.controller, item.object, purpose, hand, rule)?.first().copied())
}

/// "Exile [that card]. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {2} more to
/// cast." (CR 400.7, 601.2f): the permission is for the card's owner and lasts while it stays in exile; nothing if it did not reach
/// exile (a replacement sent it elsewhere) or is a token. Elite Spellbinder and Invasion of Gobakhan print it.
pub fn exile_owner_may_play_for_two_more(g: &mut dyn Game, card: ObjectId) {
    let owner = g.obj(card).owner;
    let exiled = g.exile(card);
    if g.obj(exiled).zone.is(ZoneKind::Exile) && !g.obj(exiled).token {
        g.allow_play(exiled, owner, PlayGrant { lands: true, more: 2, ..PlayGrant::CAST });
    }
}

/// "You may put a [kind of] card from your hand onto the battlefield" (CR 608.2d, 101.4): one card of the hand that passes
/// `eligible`, chosen as an "up to one" with the whole hand shown, then put onto the battlefield under the player's control
/// (an Aura chooses what it enchants as it enters, CR 303.4f). Nothing is asked when no card is eligible. The new object, or
/// none if nothing was put; it may have left the battlefield again by the time the caller looks. Kona, Rescue Beastie, Uro,
/// Sneak Attack, Flash and Through the Breach print it.
pub fn may_put_onto_battlefield(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    eligible: impl Fn(&Chars) -> bool,
) -> Result<Option<ObjectId>, Suspend> {
    put_from_hand(g, player, source, false, |g, c| eligible(&g.chars(c)))
}

/// `may_put_onto_battlefield` for "... from your hand onto the battlefield tapped" (CR 614.1c), the card judged as an object
/// in the hand, so what only the game can say (`Query::mana_value`: X is 0 in a hand, a Room's halves together, CR 107.3b, 202.3)
/// can decide it. Anticausal Vestige and Insidious Fungus print it.
pub fn may_put_onto_battlefield_tapped(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    eligible: impl Fn(&dyn Game, ObjectId) -> bool,
) -> Result<Option<ObjectId>, Suspend> {
    put_from_hand(g, player, source, true, eligible)
}

/// `may_put_onto_battlefield_tapped` for "... from your hand onto the battlefield tapped and attacking" (CR 508.4): the card
/// enters attacking what its controller chooses (`combat::choose_attack_target`, asked after the card), or just tapped when there
/// is nothing to attack (CR 508.4: outside combat it is only tapped). The new object, or none if nothing was put. Kinscaer
/// Sentry, Preeminent Captain, Minion of the Mighty and Shadowfax, Lord of Horses print it.
pub fn may_put_onto_battlefield_attacking(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    eligible: impl Fn(&dyn Game, ObjectId) -> bool,
) -> Result<Option<ObjectId>, Suspend> {
    let Some(card) = choose_from_hand(g, player, source, eligible)? else {
        return Ok(None);
    };
    Ok(Some(match crate::combat::choose_attack_target(g, player, source)? {
        Some(defender) => g.put_onto_battlefield_attacking(card, player, defender, &[])?,
        None => g.put_onto_battlefield(card, player, true)?,
    }))
}

fn put_from_hand(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    tapped: bool,
    eligible: impl Fn(&dyn Game, ObjectId) -> bool,
) -> Result<Option<ObjectId>, Suspend> {
    let Some(card) = choose_from_hand(g, player, source, eligible)? else {
        return Ok(None);
    };
    Ok(Some(g.put_onto_battlefield(card, player, tapped)?))
}

fn choose_from_hand(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    eligible: impl Fn(&dyn Game, ObjectId) -> bool,
) -> Result<Option<ObjectId>, Suspend> {
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let eligible: Vec<ObjectId> = hand.iter().copied().filter(|&c| eligible(&*g, c)).collect();
    Ok(choose::cards_from(g, player, source, Purpose::PutOntoBattlefield, hand, eligible, 1, true)?.first().copied())
}

/// "As this enters, look at an opponent's hand, then choose any card name." (CR 614.12a, 201.4): the opponent is chosen first
/// and the hand looked at, then the name; the name is left in `entering.chosen_card`. An unscripted player names the costliest
/// nonland card in the hand looked at. Anointed Peacekeeper and Sorcerous Spyglass print it.
pub fn look_at_an_opponents_hand_then_choose_a_name(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let you = entering.controller;
    let source = entering.object;
    let opponents: Vec<PlayerId> = g.state().opponents(you).collect();
    let Some(opponent) = choose::player(g, you, source, opponents, "LookAtHand", 0)? else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(opponent)).to_vec();
    let mut suggestion: Option<(u32, CardId)> = None;
    for &card in &hand {
        let id = g.chars(card).card;
        let def = g.card(id);
        if !def.is(CardType::Land) && suggestion.is_none_or(|(value, _)| def.cost.mana_value() > value) {
            // A split card has two names and a player names one of them (CR 709.4a): its left half's.
            let named = def.split.map_or(id, |(left, _)| g.card_id(left));
            suggestion = Some((def.cost.mana_value(), named));
        }
    }
    if let Answer::Name(name) = g.ask(you, RequestKind::ChooseName { source, looked_at: hand, suggestion: suggestion.map(|s| s.1), kind: NameKind::ANY })? {
        entering.chosen_card = name;
    }
    Ok(())
}

/// The card a discard event names and what it is - "an opponent discards a land card" (Aclazotz, Deepest Betrayal), "for
/// each nonland card discarded this way" (Seasoned Pyromancer) - read where it went (CR 701.9a): none when the event is no
/// discard, and no characteristics when a replacement put it into a hidden zone unrevealed, where every value of them is
/// undefined (CR 701.9c), so it is no card of any type, nonland included. `event` is the trigger's own, or a discard's
/// record from the history.
pub fn discarded_card(q: &dyn Query, event: &Event) -> Option<(ObjectId, Option<Chars>)> {
    match *event {
        Event::Discarded { card, undefined, .. } => Some((card, (!undefined).then(|| q.chars(card)))),
        _ => None,
    }
}

/// The discard record of `card`, the object a discard put where it went (`Game::discard_together`'s result), for
/// `discarded_card`.
pub fn discard_of<'a>(q: &'a dyn Query, card: ObjectId) -> Option<&'a Event> {
    q.history().select(EventQuery::kind(EventMask::DISCARDED).object(card)).next_back().map(|r| &r.event)
}

/// What a card a discard put where it went is (`Game::discard_together`'s result), by its discard's record
/// (`discarded_card`): none when it is undefined (CR 701.9c), or no discard put it there.
pub fn discarded_chars(q: &dyn Query, card: ObjectId) -> Option<Chars> {
    discard_of(q, card).and_then(|e| discarded_card(q, e)).and_then(|(_, chars)| chars)
}

/// "A nonland card discarded this way" (`discarded_chars`): an undefined card is no nonland card (CR 701.9c). Connive (CR
/// 701.50a), Seasoned Pyromancer, Seasoned Cryomancer, Glacial Dragonhunt and Garruk, Veiled Butcher ask it.
pub fn discarded_nonland_card(q: &dyn Query, card: ObjectId) -> bool {
    discarded_chars(q, card).is_some_and(|chars| !chars.is(CardType::Land))
}
