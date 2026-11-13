use cantrip_api::*;
use cantrip_mechanics::max_speed::itself_at_max_speed;
use cantrip_mechanics::statics::have_lifelink;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    statics: &[static_ability(Layer::Ability, itself_at_max_speed, have_lifelink, "Max speed \u{2014} This creature has lifelink.")],
    ..oracle::STREAKING_OILGORGER
};
