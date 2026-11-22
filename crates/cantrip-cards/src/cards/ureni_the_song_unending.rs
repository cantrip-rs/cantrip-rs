use cantrip_api::*;
use cantrip_mechanics::{damage_divided, lands_you_control, protection, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "X damage divided as you choose among any number of target creatures and/or planeswalkers your opponents control, where X
/// is the number of lands you control" (CR 601.2d, 603.3d, 107.3): X is the lands read as the ability is put on the stack,
/// the targets and their shares are locked in there with one damage at least on each, and what is divided stays as it was
/// whatever later changes the lands. With no land, no target can be chosen. A target that has become illegal by the time it
/// resolves is taken out with its share (CR 608.2b).
fn the_number_of_lands_you_control(q: &dyn Query, asker: &Asker, _: u32) -> u32 {
    lands_you_control(q, asker.controller) as u32
}

/// Ureni deals each target its share (CR 120.3).
fn deals_its_shares(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_divided(g, item);
    Ok(())
}

// Flying is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    statics: &[
        protection::from::<{ Color::White as u8 }>("Protection from white"),
        protection::from::<{ Color::Black as u8 }>("Protection from black"),
    ],
    abilities: &[triggered(
        THIS_ENTERS,
        deals_its_shares,
        "When Ureni enters, it deals X damage divided as you choose among any number of target creatures and/or planeswalkers your opponents control, where X is the number of lands you control.",
    )
    .targets(&[targets::CREATURE_OR_PLANESWALKER_AN_OPPONENT_CONTROLS.between(0, TargetSpec::ANY_NUMBER)])
    .divide(Divide { total: TargetSpec::COUNTED, reads: Some(the_number_of_lands_you_control) })],
    ..oracle::URENI_THE_SONG_UNENDING
};
