use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// The sacrificed artifact deals damage equal to its last-known time counters to each creature and player (CR 608.2h, 120.2).
fn explode(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount = g.obj(item.object).counters(CounterKind::Time).max(0) as u32;
    effects::damage_each_creature_and_player_n(g, item, amount);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOUR_UPKEEP, effects::counters_on_self::<{ CounterKind::Time as usize }, 1>,
            "At the beginning of your upkeep, put a time counter on this artifact."),
        ability(cost("{1}").and(TAP).and(SACRIFICE_THIS), explode,
            "{1}, {T}, Sacrifice this artifact: This artifact deals damage equal to the number of time counters on it to each creature and each player."),
    ],
    ..oracle::TIME_BOMB
};
