//! "The greatest power among creatures you control" (CR 208.1): Ghalta the Unstoppable, Go Ninja Go, Repulsive Mutation,
//! Tyvar, the Pummeler, The Great Henge and Rise of the Deathbringer ask it.

use cantrip_api::*;

/// The greatest power among the creatures `player` controls, as the characteristics are now (CR 208.1). A creature has a
/// power, so a creature whose power is not a number is not one to count; with no creature, or none above 0, the amount is
/// 0, since a negative power used as an amount counts as 0 (CR 107.1b).
pub fn among_creatures_you_control(q: &dyn Query, player: PlayerId) -> u32 {
    q.permanents_of(player)
        .into_iter()
        .map(|id| q.chars(id))
        .filter(|c| c.is(CardType::Creature))
        .filter_map(|c| c.power)
        .max()
        .unwrap_or(0)
        .max(0) as u32
}

/// "Each of `players` sacrifices a creature with the greatest power among creatures they control" (CR 701.21a, 208.1, 101.4):
/// in the order given, each player still in the game picks one of their creatures tied for the greatest power as it is now (a
/// creature that left the battlefield reads its last known power), and all the picks are sacrificed together as one action.
/// Gix's Command (each opponent) and Consumed by Greed (target opponent) ask it.
pub fn sacrifice_one_with_the_greatest_power(g: &mut dyn Game, source: ObjectId, players: &[PlayerId]) -> Result<(), Suspend> {
    let mut doomed: Vec<(PlayerId, ObjectId)> = Vec::new();
    for &who in players {
        if !crate::players::reached(g, who) {
            continue;
        }
        let mine: Vec<(ObjectId, i32)> = g
            .permanents_of(who)
            .into_iter()
            .filter(|&id| g.chars(id).is(CardType::Creature))
            .map(|id| (id, crate::targets::power_or_last_known(g, id)))
            .collect();
        let Some(greatest) = mine.iter().map(|&(_, p)| p).max() else { continue };
        let eligible: Vec<ObjectId> = mine.iter().filter(|&&(_, p)| p == greatest).map(|&(id, _)| id).collect();
        let picked = crate::choose::select(g, who, source, Purpose::Sacrifice, eligible, crate::select::Selection::new(crate::select::Count::Exactly(1)))?;
        if let Some(&card) = picked.first() {
            doomed.push((who, card));
        }
    }
    g.sacrifice_each(&doomed);
    Ok(())
}
