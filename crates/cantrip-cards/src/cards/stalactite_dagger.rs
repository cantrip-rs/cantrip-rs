use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{all_creature_types, plus_pt};
use cantrip_mechanics::tokens::{self, SHAPESHIFTER};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Equipped creature gets +1/+1 and is all creature types." (CR 613.1d, 613.4c): two layers' worth, the types in layer 4 and the
/// bonus in layer 7c, of the one creature the Equipment is attached to.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| tokens::create(g, item.controller, &SHAPESHIFTER, 1).map(drop), "When this Equipment enters, create a 1/1 colorless Shapeshifter creature token with changeling."),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[
        static_ability(Layer::Type, attached_object, all_creature_types, "Equipped creature is all creature types."),
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
    ],
    ..oracle::STALACTITE_DAGGER
};

pub static EXTRA: &[&CardDef] = &[&SHAPESHIFTER];
