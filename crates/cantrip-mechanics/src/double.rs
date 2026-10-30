//! Double (CR 701.10): to double the number of counters of a kind on a permanent is to put as many more of them on it as
//! it has (CR 701.10e, 122.8); to double a creature's power is to give it +X/+0 for the power X it has now, a negative
//! power getting -X/-0 (CR 701.10a-c); to double a player's life total is to gain or lose life until it is twice what it
//! was (CR 701.10d). Bristly Bill, Spine Sower, Court of Garenbrig and Scythecat Cub double +1/+1 counters; Legion
//! Leadership doubles power; Epic Fight and World War Hulk double power and toughness; Live Long and Prosper doubles a
//! life total.

use cantrip_api::*;

/// Double the number of `kind` counters on each of `permanents`, each one's count read before any is doubled, and all the
/// counters put on at once (CR 608.2, 101.4: one event, CR 603.2c).
pub fn counters(g: &mut dyn Game, permanents: &[ObjectId], kind: CounterKind) {
    let puts: Vec<(ObjectId, CounterKind, i16)> =
        permanents.iter().map(|&id| (id, kind, g.obj(id).counters(kind))).filter(|&(_, _, n)| n > 0).collect();
    g.add_counters_together(&puts);
}

/// Double the number of `kind` counters on each creature `player` controls.
pub fn counters_on_creatures_you_control(g: &mut dyn Game, player: PlayerId, kind: CounterKind) {
    let creatures: Vec<ObjectId> =
        g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    counters(g, &creatures, kind);
}

/// Double `creature`'s power until end of turn, the power it has now read once and fixed (CR 608.2h).
pub fn power(g: &mut dyn Game, source: ObjectId, creature: ObjectId) {
    let power = g.chars(creature).power.unwrap_or(0);
    g.add_effect(source, creature, EffectKind::Pump(power, 0), Duration::EndOfTurn);
}

/// Double `player`'s life total (CR 701.10d): they gain or lose an amount of life such that their new life total is twice
/// its current value, so a player at 0 or less loses life equal to their life total's size (CR 107.1b). The gain or loss is
/// one event, which the effects that replace it see (Alhammarret's Archive, Boon Reflection, "players can't gain life").
pub fn life_total(g: &mut dyn Game, player: PlayerId) {
    let life = g.state().player(player).life;
    if life >= 0 {
        g.gain_life(player, life);
    } else {
        g.lose_life(player, -life);
    }
}

/// Double `creature`'s power and toughness until end of turn (CR 701.10a-b): +X/+Y for the power X and toughness Y it has
/// now, each read once and fixed (CR 608.2h); a negative value gives as much less (CR 701.10c: -X/+Y, +X/-Y or -X/-Y).
/// Epic Fight and World War Hulk print it.
pub fn power_and_toughness(g: &mut dyn Game, source: ObjectId, creature: ObjectId) {
    let chars = g.chars(creature);
    let (power, toughness) = (chars.power.unwrap_or(0), chars.toughness.unwrap_or(0));
    g.add_effect(source, creature, EffectKind::Pump(power, toughness), Duration::EndOfTurn);
}
