//! "The greatest power among creatures you control" (CR 208.1): Ghalta, the Unstoppable, Go Ninja Go, Repulsive Mutation and
//! Tyvar, the Pummeler ask it.

use cantrip_api::*;

/// The greatest power among the creatures `player` controls, as the characteristics are now (CR 208.1). A creature has a
/// power, so a creature whose power is not a number is not one to count; with no creature, or none above 0, the amount is
/// 0, since a negative power used as an amount counts as 0 (CR 107.1b).
pub fn among_creatures_you_control(q: &dyn Query, player: PlayerId) -> u16 {
    q.permanents_of(player)
        .into_iter()
        .map(|id| q.chars(id))
        .filter(|c| c.is(CardType::Creature))
        .filter_map(|c| c.power)
        .max()
        .unwrap_or(0)
        .max(0) as u16
}
