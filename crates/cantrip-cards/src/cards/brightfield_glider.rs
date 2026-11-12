use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::saddle;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_oracle as oracle;

/// "Vigilance" comes with the generated keywords. "Saddle 3" (CR 702.171a) is the shared mechanic; "attacks while saddled" is
/// judged when the Glider attacks (CR 702.171b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            effects::this_gets_and_gains_flying::<1, 2>,
            "Whenever this creature attacks while saddled, it gets +1/+2 and gains flying until end of turn.",
        ),
        saddle::ability(3, "Saddle 3 (Tap any number of other creatures you control with total power 3 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::BRIGHTFIELD_GLIDER
};
