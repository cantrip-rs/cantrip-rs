//! Expend (CR 700.14): "A player expends N if they pay a cost to cast a spell and the amount of mana that player spent this
//! turn to cast spells prior to paying that cost was less than N and became at least N after paying that cost."
//!
//! The mana a player spent to cast spells is asked of the history (`History::expended`), from the payments' own records
//! (`Event::ManaSpent`, the same ones "mana spent to cast it" reads), as the spell whose payment it is becomes cast. So it
//! counts only mana actually spent, of every cost paid to cast, on a spell later countered too; nothing spent on an
//! ability, and nothing for a spell cast without spending mana. One payment can expend several numbers at once (expend 4
//! and expend 8), and each triggers once a turn at most, since the total only grows. A permanent that enters after its
//! controller spent their fourth mana doesn't see that payment: its ability triggers only on a later one that crosses N.

use cantrip_api::*;

/// "Whenever you expend N" (CR 700.14, 603.2): the payment for a spell its controller just cast took the mana they spent
/// to cast spells this turn from below `N` to `N` or more. Asked with the ability's controller ("you").
pub fn you_expend<const N: u32>(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = *event else { return None };
    if player != me.controller {
        return None;
    }
    let (before, with) = q.history().expended(player, spell);
    (before < N && with >= N).then_some(Fired { subject: Target::Player(player), data: [0; 2] })
}

/// The trigger "whenever you expend N" (CR 700.14).
pub const fn expend<const N: u32>() -> Trigger {
    when_asked(EventMask::SPELL_CAST, you_expend::<N>)
}

/// "Whenever you expend 4".
pub const EXPEND_4: Trigger = expend::<4>();
