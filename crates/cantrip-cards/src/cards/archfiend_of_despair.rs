use cantrip_api::*;
use cantrip_mechanics::statics::opponents_cant_gain_life;
use cantrip_mechanics::triggers::EACH_END_STEP;
use cantrip_oracle as oracle;

/// "At the beginning of each end step, each opponent loses life equal to the life that player lost this turn." (CR 119.3,
/// 603.2b.) Each opponent's own total is read as the ability resolves, so a second Archfiend resolving after this one sees
/// the loss this one caused, and life gained does not make the total smaller (rulings 2018-06-08). The controller is not
/// an opponent and loses nothing.
fn each_opponent_loses_again(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    for opponent in opponents {
        let lost = g.history().life_lost(opponent);
        g.lose_life(opponent, i64::from(i32::try_from(lost).unwrap_or(i32::MAX)));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cant_gain_life: &[opponents_cant_gain_life],
    abilities: &[triggered(
        EACH_END_STEP,
        each_opponent_loses_again,
        "At the beginning of each end step, each opponent loses life equal to the life that player lost this turn. (Damage causes loss of life.)",
    )],
    ..oracle::ARCHFIEND_OF_DESPAIR
};
