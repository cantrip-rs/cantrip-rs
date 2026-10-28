//! Amass [subtype] N (CR 701.47a): "Put N +1/+1 counters on an Army you control. If you don't control an Army, create a
//! 0/0 black [subtype] Army creature token first. The Army becomes a [subtype] in addition to its other types."
//! Orcish Bowmasters amasses Orcs 1.

use crate::choose;
use cantrip_api::*;

/// Amass `subtype` `n`: `token` is the 0/0 black `subtype` Army creature token created when `controller` has no Army.
/// `subtype` is the word the card prints, read through `source`'s text (CR 612.2): the rest of the Army is the keyword
/// action's own (CR 701.47a), which no text change reaches.
pub fn amass(
    g: &mut dyn Game,
    controller: PlayerId,
    source: ObjectId,
    token: &'static CardDef,
    subtype: Subtype,
    n: i16,
) -> Result<(), Suspend> {
    amass_army(g, controller, source, token, subtype, n).map(|_| ())
}

/// `amass`, and the Army the player chose to receive the counters, "the amassed Army" (CR 701.47c): whether or not it received
/// counters. `None` if there was none to choose. Goblin Plate Mail attaches itself to it; Foray of Orcs, Grishnakh, Brash Instigator
/// and Surrounded by Orcs read its power.
pub fn amass_army(
    g: &mut dyn Game,
    controller: PlayerId,
    source: ObjectId,
    token: &'static CardDef,
    subtype: Subtype,
    n: i16,
) -> Result<Option<ObjectId>, Suspend> {
    let armies = |g: &dyn Game| -> Vec<ObjectId> {
        g.permanents_of(controller)
            .into_iter()
            .filter(|&id| {
                let c = g.chars(id);
                c.is(CardType::Creature) && c.has_subtype(Subtype::Army)
            })
            .collect()
    };
    let printed = subtype;
    let subtype = g.words(source).subtype(printed);
    let mut candidates = armies(g);
    if candidates.is_empty() {
        let token = g.card_id(token);
        let words = Words::NONE.changed(Word::Subtype(printed), Word::Subtype(subtype));
        g.create_tokens_worded(&[(token, controller)], false, None, words)?;
        candidates = armies(g);
    }
    let army = match candidates.as_slice() {
        [only] => *only,
        // Several Armies: which gets the counters is chosen, not targeted (CR 701.47a).
        _ => match choose::cards_from_kind(g, controller, source, Purpose::PutCounters(CounterKind::PlusOne, n.clamp(0, 255) as u8), "Army", candidates.clone(), candidates, 1, false)?.first() {
            Some(&army) => army,
            None => return Ok(None),
        },
    };
    g.add_counters(army, CounterKind::PlusOne, n);
    if !g.chars(army).has_subtype(subtype) {
        g.add_effect(source, army, EffectKind::AddSubtype(subtype), Duration::Indefinite);
    }
    Ok(Some(army))
}

/// "Amass Goblins N" as a whole ability or spell effect: the Goblin Army token is `tokens::GOBLIN_ARMY` (CR 701.47a).
/// Down, Down to Goblin-town, Bothersome Noisemaker, Goblin-town Flunkies and Misty Mountains Raider print it with
/// their own `N`.
pub fn amass_goblins<const N: i16>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    amass(g, item.controller, item.object, &crate::tokens::GOBLIN_ARMY, Subtype::Goblin, N)
}
