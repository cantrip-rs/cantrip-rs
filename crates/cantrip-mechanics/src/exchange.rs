//! Exchange (CR 701.12): the keyword action, for life totals (CR 701.12c) and for control of two targets (CR 701.12b).

use cantrip_api::*;

/// `a` and `b` exchange life totals (CR 701.12c): each gains or loses the amount of life needed to equal the other's previous
/// total, and replacement effects may modify those gains and losses. If the entire exchange can't be completed, no part of it
/// occurs (CR 701.12a): a player who can't gain life can't be given a higher total, and one who can't lose life can't be given
/// a lower one (CR 119.7, 119.8). The life `a` lost this way, by the totals before and after (none if `a` gained life, nothing
/// was exchanged, or a replacement effect stopped the loss).
pub fn life_totals(g: &mut dyn Game, a: PlayerId, b: PlayerId) -> u32 {
    let (before_a, before_b) = (g.state().player(a).life, g.state().player(b).life);
    let (gainer, loser) = if before_a < before_b { (a, b) } else { (b, a) };
    if !g.can_gain_life(gainer) || !g.can_lose_life(loser) {
        return 0;
    }
    // abs_diff is a u32 and covers the full span between any two i32 totals.
    let difference = before_a.abs_diff(before_b);
    // One simultaneous change of both life totals (CR 701.12c, 101.4): replaced in APNAP order before either changes, and
    // announced together.
    g.life_changes_together(&[
        LifeChange { player: loser, magnitude: u64::from(difference), negative: true },
        LifeChange { player: gainer, magnitude: u64::from(difference), negative: false },
    ]);
    let after_a = g.state().player(a).life;
    if before_a > after_a { before_a.abs_diff(after_a) } else { 0 }
}

/// "Exchange control of [target A] and [target B]" (CR 701.12a-b) for a spell or ability whose targets 0 and 1 are the two
/// permanents: both must still be legal targets as it resolves, or the exchange can't be completed and none of it happens; two
/// permanents one player controls exchange nothing. Auras and Equipment attached to them stay with their controllers.
/// Switcheroo, Oko, Thief of Crowns, Bilbo's Burglar's Plot and Trade the Helm print it.
pub fn control_of_two_targets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let (Some(first), Some(second)) = (crate::targets::object(g, item, 0), crate::targets::object(g, item, 1)) {
        g.exchange_control(item.object, first, second);
    }
    Ok(())
}
