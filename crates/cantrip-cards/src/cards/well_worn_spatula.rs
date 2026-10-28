use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::effects::gain_life;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this Equipment enters, you gain 2 life." (CR 119.3), "Equipped creature gets +1/+1." and "Equip {1}" (CR 702.6a).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| gain_life(g, item, 2), "When this Equipment enters, you gain 2 life."),
        equip(mana("{1}"), "Equip {1}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    ..oracle::WELL_WORN_SPATULA
};
