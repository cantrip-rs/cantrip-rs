//! Suspend (CR 702.62a): "Rather than cast this card from your hand, you may pay [cost] and exile it with N time
//! counters on it. At the beginning of your upkeep, remove a time counter from it. When the last is removed, cast it
//! without paying its mana cost." Lotus Bloom and Rift Bolt print it.
//!
//! A card with suspend sets `suspend: Some(SuspendDef { count, cost })` and lists `ability(cost, text)`: the special
//! action (CR 116.2) of exiling it from the hand whenever it could be cast. Printing suspend gives the card the keyword
//! (`Keyword::Suspend`, `CardDef::printed_keywords`); a card that gains suspend (Delay, Epochrasite) is given it by an effect
//! (`EffectKind::AddKeywords`). Either way the layers give an object with the keyword suspend's two triggered abilities,
//! which function in exile (`cantrip_api::intrinsic::SUSPEND_UPKEEP`, `SUSPEND_CAST`), and so is the haste a creature spell
//! cast by the second gains until its caster loses control of it (`intrinsic::SUSPEND_HASTE`, carried to the permanent it
//! becomes, CR 400.7a).

use cantrip_api::*;

/// "Exile [this card] with N time counters on it" (CR 702.62a, 122.1): the card is exiled and, only if it really is in exile
/// afterwards (a replacement effect may have sent it elsewhere), the counters go on the object it became there. The special
/// action of suspend and the resolving spells that print it - Chronomantic Escape and Rousing Refrain, Arc Blade, Cyclical
/// Evolution - exile their own card this way, and so does Taigam's flurry for the spell it is about. The object in exile, or
/// nothing if the card did not end up there.
pub fn exile_with_time_counters(g: &mut dyn Game, card: ObjectId, count: u8) -> Option<ObjectId> {
    let exiled = g.exile(card);
    if !g.obj(exiled).zone.is(ZoneKind::Exile) {
        return None;
    }
    g.add_counters(exiled, CounterKind::Time, i32::from(count));
    Some(exiled)
}

fn suspend(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = g.card(g.obj(item.object).card).suspend.map_or(0, |s| s.count);
    exile_with_time_counters(g, item.object, count);
    Ok(())
}

pub const fn ability(cost: ManaCost, text: &'static str) -> AbilityDef {
    cantrip_api::ability(Cost::mana(cost), suspend, text)
        .timing(Timing::AsCast)
        .from(ZoneKind::Hand)
        .special_action()
}
