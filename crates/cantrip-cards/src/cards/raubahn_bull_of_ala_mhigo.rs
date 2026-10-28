use cantrip_api::*;
use cantrip_mechanics::attach::attach_first_target_to_second;
use cantrip_mechanics::triggers::{THIS_ATTACKS, ward_life_equal_to_power};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

const WARD: AbilityDef = ward_life_equal_to_power("Ward\u{2014}Pay life equal to Raubahn's power. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays life equal to its power.)");

pub static CARD: CardDef = CardDef {
    abilities: &[
        WARD,
        triggered(THIS_ATTACKS, attach_first_target_to_second, "Whenever Raubahn attacks, attach up to one target Equipment you control to target attacking creature.")
            .targets(&[
                up_to_one(targets::EQUIPMENT_YOU_CONTROL.filter).labelled("Equipment to attach"),
                targets::ATTACKING_CREATURE.labelled("Attacking creature to attach it to"),
            ]),
    ],
    ..oracle::RAUBAHN_BULL_OF_ALA_MHIGO
};
