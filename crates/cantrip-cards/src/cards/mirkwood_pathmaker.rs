use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// "Mirkwood Pathmaker's power and toughness are each equal to the number of lands you control" (CR 604.3, layer 7a): a
/// characteristic-defining ability, so it works in every zone, counting its owner's lands off the battlefield.
pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(
        Layer::PtDefine,
        lands::power_and_toughness_are_lands_you_control,
        "Mirkwood Pathmaker's power and toughness are each equal to the number of lands you control.",
    )
    .lost_pt(0, 0)],
    ..oracle::MIRKWOOD_PATHMAKER
};
