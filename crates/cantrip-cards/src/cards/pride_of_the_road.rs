use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature or Vehicle you control gains double strike until end of turn" (CR 611.2a, 702.4): a layer 6 effect of
/// this ability on the target, if it is still a legal target as the ability resolves (CR 608.2b).
fn double_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, target, Keyword::DoubleStrike);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    delayed: &[triggered(BEGINNING_OF_YOUR_COMBAT, double_strike, "At the beginning of combat on your turn, target creature or Vehicle you control gains double strike until end of turn.")
        .targets(&[targets::CREATURE_OR_VEHICLE_YOU_CONTROL])],
    statics: &[max_speed("Max speed \u{2014} At the beginning of combat on your turn, target creature or Vehicle you control gains double strike until end of turn.")],
    ..oracle::PRIDE_OF_THE_ROAD
};
