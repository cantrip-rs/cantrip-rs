//! Backup N (CR 702.165a): "When this creature enters, put N +1/+1 counters on target creature. If that's another
//! creature, it gains the following abilities until end of turn." Death Greeter's Champion and Guardian Scalelord print
//! it.
//!
//! A card with backup lists `trigger(text, effect)` among its `abilities`, `effect` calling `resolve`. The abilities
//! granted are the printed ones, fixed when backup triggers (CR 702.165d): the keywords it names, abilities of the kind
//! `Keyword` holds included (`Keyword::AssignsDamageByToughness`), and, if it lists the creature's own ability as a delayed
//! ability, that one.

use crate::targets;
use crate::triggers::THIS_ENTERS;
use cantrip_api::*;

/// The backup trigger: it enters, targeting a creature; `effect` resolves it with `resolve`.
pub const fn trigger(text: &'static str, effect: fn(&mut dyn Game, &StackItem) -> Result<(), Suspend>) -> AbilityDef {
    triggered(THIS_ENTERS, effect, text)
        .targets(&[targets::CREATURE])
}

/// Put `n` +1/+1 counters on the target creature; if that is another creature, it gains `keywords` and, if given, the
/// ability the card lists as its delayed ability `ability`, until end of turn.
pub fn resolve(g: &mut dyn Game, item: &StackItem, n: i16, keywords: &[Keyword], ability: Option<u8>) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Target::Object(creature) = item.target(0)
    {
        g.add_counters(creature, CounterKind::PlusOne, i64::from(n));
        if creature != item.object {
            // "It gains the following abilities": one instruction (`Game::one_instruction`).
            g.one_instruction(&mut |g| {
                g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(keywords)), Duration::EndOfTurn);
                if let Some(index) = ability {
                    // A copy's ability is the one of the card it copies (CR 707.2).
                    let o = g.obj(item.object);
                    let this = o.copied_card().unwrap_or(o.card);
                    g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, index)), Duration::EndOfTurn);
                }
                Ok(())
            })?;
        }
    }
    Ok(())
}
