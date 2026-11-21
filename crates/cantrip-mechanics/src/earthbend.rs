//! Earthbend N (CR 701.66a): "Target land you control becomes a 0/0 land creature with haste in addition to its other
//! types. Put N +1/+1 counters on it. When that land dies or is put into exile, return it to the battlefield tapped under
//! your control." Badgermole Cub, Ba Sing Se, Earthbender Ascension, Aang, at the Crossroads and Dai Li Indoctrination
//! print it.
//!
//! A card that earthbends lists `enters::<N>(text)` (or `TARGET` and `effect::<N>` for a spell or another ability) and
//! `RETURN` as its first `delayed` ability: the delayed triggered ability earthbending creates (CR 603.7), which watches
//! the land (`DelayedWhen::WhenEvent`). It is no ability of the land: losing its abilities (Turn to Frog) doesn't stop it,
//! and it is controlled by the earthbender, whoever controls the land when it dies (CR 603.7d-e).

use crate::triggers::THIS_ENTERS;
use cantrip_api::*;

/// "Target land you control."
pub const TARGET: TargetSpec = crate::targets::LAND_YOU_CONTROL;

/// The land becomes a 0/0 creature with haste that's still a land, with no duration (CR 611.2a); it gets `N` +1/+1
/// counters; and `RETURN`, the delayed ability 0 of the card whose ability this is (`StackItem::ability`: the face that
/// prints it, whatever card the object has now - a merged creature's top, CR 702.140e), is created, watching it: its
/// source is this spell or this ability's source, its controller this spell or ability's (CR 603.7d-e).
pub fn effect<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    earthbend(g, item, N)
}

/// "Earthbend X" (CR 701.66a, 107.3): `effect` for a number worked out as the spell or ability resolves (CR 608.2h), the instruction the
/// same otherwise. Rockalanche (Forests you control), The Boulder, Ready to Rumble (creatures you control with power 4 or greater) and
/// The Legend of Kyoshi (cards in your hand) print it.
pub fn earthbend(g: &mut dyn Game, item: &StackItem, n: i32) -> Result<(), Suspend> {
    earthbend_target(g, item, 0, n)
}

/// "Earthbend N" of the land chosen for target slot `slot` (CR 115.1, 608.2b): an ability that earthbends more than once names a
/// "target land you control" for each (CR 115.3: the same land may be chosen for each), and each instruction does nothing if its
/// own target is no longer legal. Dai Li Agents' "earthbend 1, then earthbend 1" is slots 0 and 1.
pub fn earthbend_target(g: &mut dyn Game, item: &StackItem, slot: usize, n: i32) -> Result<(), Suspend> {
    if !g.target_legal(item, slot) {
        return Ok(());
    }
    let Target::Object(land) = item.target(slot) else { return Ok(()) };
    // "It becomes a 0/0 land creature with haste": one instruction (`Game::one_instruction`).
    g.one_instruction(&mut |g| {
        g.add_effect(item.object, land, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::Indefinite);
        crate::effects::gain_haste(g, item.object, land);
        g.add_effect(item.object, land, EffectKind::SetPt(0, 0), Duration::Indefinite);
        Ok(())
    })?;
    g.add_counters(land, CounterKind::PlusOne, n);
    g.delayed_trigger(DelayedWhen::WhenEvent, item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), Target::Object(land));
    Ok(())
}

/// "When this enters, earthbend N": `text` names the card.
pub const fn enters<const N: i32>(text: &'static str) -> AbilityDef {
    triggered(THIS_ENTERS, effect::<N>, text)
        .targets(&[TARGET])
}

/// "When that land dies or is put into exile" (CR 700.4): the land it watches leaving the battlefield for a graveyard or
/// exile, judged as it was (CR 603.10a).
fn dies_or_is_exiled(_: &dyn Query, land: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if *old == land && from.is(ZoneKind::Battlefield) && (to.is(ZoneKind::Graveyard) || to.is(ZoneKind::Exile)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Return it to the battlefield tapped under your control": the card the land became, if it is still where it went (CR
/// 400.7e, 603.7c), under the ability's controller - each card of a merged permanent, together (CR 730.3c).
fn return_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(land) = item.subject else { return Ok(()) };
    let cards: Vec<(ObjectId, PlayerId, bool)> = g.found_all_after_move(land).into_iter().map(|c| (c, item.controller, true)).collect();
    if !cards.is_empty() {
        g.put_all_onto_battlefield_as(&cards)?;
    }
    Ok(())
}

/// The delayed triggered ability earthbending creates (`effect`), about the land (`StackItem::subject`).
pub const RETURN: AbilityDef = triggered(
    when(EventMask::ZONE_CHANGE, dies_or_is_exiled),
    return_tapped,
    "When that land dies or is put into exile, return it to the battlefield tapped under your control.",
);
