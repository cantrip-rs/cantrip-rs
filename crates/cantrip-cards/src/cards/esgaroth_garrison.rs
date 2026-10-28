use cantrip_api::*;
use cantrip_mechanics::{recruit, statics, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(Layer::PtDefine, statics::power_is_creatures_you_control, "Esgaroth Garrison's power is equal to the number of creatures you control.").lost_pt(0, 5)],
    abilities: &[triggered(triggers::THIS_ENTERS, recruit::effect, "When this creature enters, recruit.")],
    ..oracle::ESGAROTH_GARRISON
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN_SOLDIER];
