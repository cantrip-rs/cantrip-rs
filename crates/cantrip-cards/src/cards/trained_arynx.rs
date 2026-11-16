use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_mechanics::{effects, saddle};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks while saddled, it gains first strike until end of turn. Scry 1." (CR 702.7a, 701.22): two
/// instructions in order; the scry follows whether or not the Arynx is still there to gain anything.
fn gains_first_strike_then_scry_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::this_gains_first_strike(g, item)?;
    effects::scry(g, item, 1)
}

/// "Saddle 2" (CR 702.171a) is the shared mechanic; "attacks while saddled" is judged when the Arynx attacks (CR 702.171b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            gains_first_strike_then_scry_one,
            "Whenever this creature attacks while saddled, it gains first strike until end of turn. Scry 1.",
        ),
        saddle::ability(2, "Saddle 2 (Tap any number of other creatures you control with total power 2 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::TRAINED_ARYNX
};
