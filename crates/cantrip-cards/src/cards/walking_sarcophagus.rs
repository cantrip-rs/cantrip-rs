use cantrip_api::*;
use cantrip_mechanics::max_speed::itself_at_max_speed;
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    // "Max speed — This creature gets +1/+2": as long as its controller's speed is 4 (CR 702.178a), a power and toughness
    // change in layer 7c (CR 613.4c).
    statics: &[static_ability(Layer::PtModify, itself_at_max_speed, plus_pt::<1, 2>, "Max speed \u{2014} This creature gets +1/+2.")],
    ..oracle::WALKING_SARCOPHAGUS
};
