use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, attach::attached_object, statics::have_flying_and_first_strike, "Enchanted creature gets +1/+0 and has flying and first strike."),
        static_ability(Layer::PtModify, attach::attached_object, statics::plus_pt::<1, 0>, "Enchanted creature gets +1/+0 and has flying and first strike.").continues(),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::WINGS_OF_AESTHIR
};
