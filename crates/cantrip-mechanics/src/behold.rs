//! Behold (CR 701.4a): "choose a [quality] permanent you control or reveal a [quality] card from your hand", used here as
//! an additional cost ("as an additional cost to cast this spell, you may behold a Dragon"). Dispelling Exhale is in the
//! engine; Piercing Exhale and Osseous Exhale print it as an optional cost, and Caustic Exhale, Kinsbaile Aspirant and
//! Silvergill Mentor print it as a choice with another cost.
//!
//! "As this land enters, you may behold a [quality]" (Theorist's Sanctum) is `as_enters`: the same choice made as a
//! replacement effect applies (CR 614.12a), not as a cost. "You may behold a [quality]" in an effect (Sarkhan, Dragon
//! Ascendant) is `on_resolution`.
//!
//! Choosing the permanent or revealing the card is the core's (`Cost::behold`, paid as the other costs are, CR 601.2h);
//! nothing moves. A card with the optional kind gives `optional(filter)` as its `optional_cost`, and "if a Dragon was
//! beheld" is `beheld` of the resolving spell: the cost is its only way to behold, so it was paid or not (CR 601.2b).

use cantrip_api::*;

/// The additional cost "behold a [quality]" as a part of a cost, the quality named as the prompt says it ("Dragon").
pub const fn cost(filter: ObjectFilter, quality: &'static str) -> Cost {
    Cost { behold: Some((1, filter, quality)), ..Cost::NONE }
}

/// "As an additional cost to cast this spell, you may behold a [quality]."
pub const fn optional(filter: ObjectFilter, quality: &'static str) -> OptionalCost {
    OptionalCost { cost: cost(filter, quality) }
}

/// "If a [quality] was beheld": the spell's optional additional cost was paid. A copy of the spell has it paid when the
/// original did (CR 707.10).
pub fn beheld(item: &StackItem) -> bool {
    item.paid > 0
}

/// A Dragon, read through the words of the card whose text says it (CR 612): a permanent or a card in a hand.
pub fn dragon(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Dragon))
}

/// A Goblin, read through the words of the card whose text says it (CR 612): a permanent or a card in a hand. Mudbutton
/// Cursetosser and Champion of the Weird behold one.
pub fn goblin(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Goblin))
}

/// A Jace, a permanent or a card in a hand: a planeswalker type is not a word a text-changing effect changes (CR 612.1,
/// 205.3j). Theorist's Sanctum and Countersculpt behold one.
pub fn jace(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(Subtype::Jace)
}

/// A [quality] permanent `player` controls or card in their hand that passes `filter` (except `except`) is chosen, if they
/// have one and want to (CR 701.4a); a card from the hand is revealed to every player. Whether they beheld. Nobody is
/// asked when there is nothing to behold.
fn behold_one(g: &mut dyn Game, player: PlayerId, source: ObjectId, except: Option<ObjectId>, filter: ObjectFilter, quality: &'static str) -> Result<bool, Suspend> {
    let asker = Asker { source, controller: player, by: None, spell: false };
    let mut eligible = g.permanents_of(player);
    eligible.extend(g.state().zone(Zone::hand(player)).iter().copied());
    eligible.retain(|&id| Some(id) != except && filter(&*g, &asker, id));
    let chosen = crate::choose::cards_from_kind(g, player, source, Purpose::Behold, quality, eligible.clone(), eligible, 1, true)?;
    let Some(&card) = chosen.first() else { return Ok(false) };
    if g.obj(card).zone.is(ZoneKind::Hand) {
        g.reveal(player, &[card]);
    }
    Ok(true)
}

/// "As this permanent enters, you may behold a [quality]" (CR 701.4a, 614.12a): the entering permanent's controller may
/// choose a permanent they control or reveal a card in their hand that passes `filter`, the entering card apart. Whether
/// they beheld. Nobody is asked when they have nothing to behold. Theorist's Sanctum prints it with a Jace.
pub fn as_enters(g: &mut dyn Game, entering: &Entering, filter: ObjectFilter, quality: &'static str) -> Result<bool, Suspend> {
    behold_one(g, entering.controller, entering.object, Some(entering.object), filter, quality)
}

/// "You may behold a [quality]" as a spell or ability resolves (CR 701.4a, 608.2d): `player` may choose a permanent they
/// control or reveal a card in their hand that passes `filter`, read as the source's text says it. Whether they beheld; "if
/// you do" follows it. Nobody is asked when they have nothing to behold. Sarkhan, Dragon Ascendant prints it with a Dragon.
pub fn on_resolution(g: &mut dyn Game, player: PlayerId, source: ObjectId, filter: ObjectFilter, quality: &'static str) -> Result<bool, Suspend> {
    behold_one(g, player, source, None, filter, quality)
}
