//! Exchange (CR 701.12): the keyword action, here for life totals (CR 701.12c).

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
    let difference = (before_a - before_b).abs();
    g.lose_life(loser, difference);
    g.gain_life(gainer, difference);
    (before_a - g.state().player(a).life).max(0).unsigned_abs()
}
