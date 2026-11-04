use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "Snow Villiers's power is equal to the number of creatures you control." (CR 604.3, 613.4a): a characteristic-defining
    // ability that counts Snow Villiers itself while it is a creature on the battlefield (the ruling).
    statics: &[characteristic_defining(Layer::PtDefine, statics::power_is_creatures_you_control, "Snow Villiers's power is equal to the number of creatures you control.").lost_pt(0, 3)],
    ..oracle::SNOW_VILLIERS
};
