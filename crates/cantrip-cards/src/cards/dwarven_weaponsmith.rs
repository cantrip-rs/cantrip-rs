use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{T}, Sacrifice an artifact: Put a +1/+1 counter on target creature. Activate only during your upkeep." (CR 602.5).
fn put_1_1_counter_target_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.controller)
        .is_some_and(|player: PlayerId| {
            q.state().turn.active == player && q.state().turn.step() == Step::Upkeep
        })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP.and(sacrifice(1, targets::filters::artifact)),
                effects::plus_one_counter_on_target,
                "{T}, Sacrifice an artifact: Put a +1/+1 counter on target creature. Activate only during your upkeep.",
            )
            .targets(&[targets::CREATURE])
            .condition(put_1_1_counter_target_if),
    ],
    ..oracle::DWARVEN_WEAPONSMITH
};
