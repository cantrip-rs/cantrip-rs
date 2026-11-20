use cantrip_api::*;
use cantrip_mechanics::{effects, history, targets, triggers::BEGINNING_OF_YOUR_COMBAT};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+0 and gains menace until end of turn" (CR 611.2a, 702.111).
fn target_gets_and_gains_menace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::targets_get_and_gain(g, item, 1, 0, Keyword::Menace)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        target_gets_and_gains_menace,
        "Celebration \u{2014} At the beginning of combat on your turn, if two or more nonland permanents entered the battlefield under your control this turn, target creature you control gets +1/+0 and gains menace until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])
    .condition(history::you_celebrate)],
    ..oracle::BELLIGERENT_OF_THE_BALL
};
