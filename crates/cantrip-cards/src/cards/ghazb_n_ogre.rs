use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// The sole player with more life than every other player still in the game (CR 800.4a).
fn life_leader(q: &dyn Query) -> Option<PlayerId> {
    let leader = q.state().apnap().max_by_key(|&p| q.state().player(p).life)?;
    let life = q.state().player(leader).life;
    q.state().apnap().all(|p| p == leader || q.state().player(p).life < life).then_some(leader)
}

/// The intervening condition is checked both when the ability triggers and as it resolves (CR 603.4).
fn has_life_leader(q: &dyn Query, _: &Asker) -> bool {
    life_leader(q).is_some()
}

/// The current unique leader gains control indefinitely (CR 611.2a).
fn gain_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // The intervening condition was just rechecked (CR 603.4): with no unique leader, nothing happens.
    let Some(leader) = life_leader(g) else { return Ok(()) };
    g.gain_control(item.object, item.object, leader, Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            gain_control,
            "At the beginning of your upkeep, if a player has more life than each other player, the player with the most life gains control of this creature.",
        )
        .condition(has_life_leader),
    ],
    ..oracle::GHAZB_N_OGRE
};
