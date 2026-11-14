use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, statics::other_creatures_you_control, statics::have_trample, "Other creatures you control have trample.")],
    ..oracle::AGGRESSIVE_MAMMOTH
};
