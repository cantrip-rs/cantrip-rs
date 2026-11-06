use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "This creature enters with three +1/+1 counters on it. Remove a +1/+1 counter from this creature: It deals 1 damage to any target." (CR 614.1c, 122.6).
fn deals_1_damage_any_target_enters(
    _: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    entering.counters[CounterKind::PlusOne as usize] = entering
        .counters[CounterKind::PlusOne as usize]
        .saturating_add(3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[effects::REMOVE_PLUS_ONE_TO_DEAL_ONE],
    enters: Some(deals_1_damage_any_target_enters),
    ..oracle::TRISKELION
};
