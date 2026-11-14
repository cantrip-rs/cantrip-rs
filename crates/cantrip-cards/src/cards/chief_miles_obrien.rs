use cantrip_api::*;
use cantrip_mechanics::{double, seek_out::seek_out, targets};
use cantrip_oracle as oracle;

/// "Whenever you draw your second card each turn, double the number of each kind of counter on target artifact or creature
/// you control." The shared trigger counts your draws this turn (CR 121.1, 603.2). Doubling adds as many counters of every
/// kind as the legal target has when this resolves (CR 701.10e, 608.2b, 608.2h), all put on at once.
pub static CARD: CardDef = CardDef {
    abilities: &[seek_out(
        double::each_kind_of_counter_on_each_target,
        &[targets::ARTIFACT_OR_CREATURE_YOU_CONTROL],
        "Seek out — Whenever you draw your second card each turn, double the number of each kind of counter on target artifact or creature you control.",
    )],
    ..oracle::CHIEF_MILES_OBRIEN
};
