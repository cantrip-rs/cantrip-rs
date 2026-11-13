use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(
        Layer::PtDefine,
        lands::power_and_toughness_are_creatures_you_control,
        "Crusader of Odric's power and toughness are each equal to the number of creatures you control.",
    )
    .lost_pt(0, 0)],
    ..oracle::CRUSADER_OF_ODRIC
};
