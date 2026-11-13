//! Raid (CR 207.2c, an ability word): "Raid \u{2014} When this creature enters, if you attacked this turn, [effect]." It has no
//! rules meaning of its own; "if you attacked this turn" is an intervening "if" (CR 603.4) about the declarations of attackers
//! the ability's controller made this turn (CR 508.1): one attacker is enough, whichever it was and whomever it attacked, and
//! it need not still be on the battlefield (the Raid rulings). Fire Nation Raider, Mardu Hordechief, Skyship Buccaneer and
//! Wingmate Roc print it.
//!
//! A card with a raid enters trigger lists `enters(effect, text)` among its `abilities`; one that "enters with a +1/+1 counter
//! if you attacked this turn" sets `enters: Some(raid::enters_with_a_counter)`.

use cantrip_api::*;

use crate::history::creatures_attacked_with;
use crate::triggers::THIS_ENTERS;

/// "If you attacked this turn": checked as the ability triggers and again as it resolves (CR 603.4). Fire Nation Engineer asks it
/// at the beginning of the end step.
pub fn you_attacked_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    creatures_attacked_with(q, asker.controller) > 0
}

/// "Raid \u{2014} When this creature enters, if you attacked this turn, [effect]." `text` is the ability's full text, as printed.
pub const fn enters(effect: fn(&mut dyn Game, &StackItem) -> Result<(), Suspend>, text: &'static str) -> AbilityDef {
    triggered(THIS_ENTERS, effect, text).condition(you_attacked_this_turn)
}

/// "Raid \u{2014} This creature enters with a +1/+1 counter on it if you attacked this turn" (CR 614.1c, 207.2c, 508.1): a
/// replacement effect, not a trigger, so it reads this turn's attacks as the creature enters under `entering.controller`.
/// Goblin Boarders prints it.
pub fn enters_with_a_counter(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if creatures_attacked_with(g, entering.controller) > 0 {
        entering.counters[CounterKind::PlusOne as usize] += 1;
    }
    Ok(())
}
