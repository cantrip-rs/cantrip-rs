use cantrip_api::*;
use cantrip_mechanics::statics::{have_lifelink, plus_pt};
use cantrip_mechanics::tokens::{self, PILOT_SADDLES_AND_CREWS};
use cantrip_mechanics::{attach, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_OR_VEHICLE),
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        tokens::create_a_pilot,
        "When this Aura enters, create a 1/1 colorless Pilot creature token with \"This token saddles Mounts and crews Vehicles as though its power were 2 greater.\"",
    )],
    statics: &[
        static_ability(Layer::Ability, attach::attached_object, have_lifelink, "Enchanted permanent gets +1/+1 and has lifelink."),
        static_ability(Layer::PtModify, attach::attached_object, plus_pt::<1, 1>, "Enchanted permanent gets +1/+1 and has lifelink.").continues(),
    ],
    ..oracle::ROADSIDE_ASSISTANCE
};

pub static EXTRA: &[&CardDef] = &[&PILOT_SADDLES_AND_CREWS];
