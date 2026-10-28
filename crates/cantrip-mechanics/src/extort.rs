//! Extort (CR 702.101a): "Whenever you cast a spell, you may pay {W/B}. If you do, each opponent loses 1 life and you gain
//! life equal to the total life lost this way." Each instance triggers separately (CR 702.101b). Basilica Guards, Basilica
//! Screecher, Kingpin's Pet, Knight of Obligation, Syndicate Enforcer, Syndic of Tithes and Tithe Drinker print it.
//!
//! A card with extort lists `EXTORT` among its `abilities`.

use crate::triggers::you_cast_a_spell;
use cantrip_api::*;

const PAY: Cost = of_mana(mana("{W/B}"));

const QUESTION: Question = Question::new(
    "Extort",
    "pay {cost}?",
    "If you do, each opponent loses 1 life and you gain life equal to the total life lost this way",
    "Pay {cost}",
    "Don't extort",
)
.no_default();

/// "You may pay {W/B}. If you do, each opponent loses 1 life and you gain life equal to the total life lost this way" (CR
/// 118.12, 119.3): asked as the ability resolves, once for each ability (the rulings). The life gained is what was lost in
/// all, which a player whose life total can't change loses none of, so with one such opponent and no other, nothing is gained.
/// The ability targets no one.
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, PAY, &QUESTION)? {
        let lost = g.each_opponent_loses_life(item.controller, 1);
        g.gain_life(item.controller, lost);
    }
    Ok(())
}

/// Extort, with the reminder text its cards print.
pub const EXTORT: AbilityDef = triggered(
    when_asked(EventMask::SPELL_CAST, you_cast_a_spell),
    drain,
    "Extort (Whenever you cast a spell, you may pay {W/B}. If you do, each opponent loses 1 life and you gain that much life.)",
);
