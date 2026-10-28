use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::triggers::EQUIPPED_CREATURE_ATTACKS_ALONE;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Untap it and scry 1." (CR 701.26b, 701.22a): the creature that attacked alone is untapped, if it is still the same object
/// (CR 400.7), and stays attacking; the scry is the controller's.
fn untap_and_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        g.untap(creature);
    }
    effects::scry(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(EQUIPPED_CREATURE_ATTACKS_ALONE, untap_and_scry, "Whenever equipped creature attacks alone, untap it and scry 1."),
        equip(mana("{1}"), "Equip {1}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    ..oracle::S_H_I_E_L_D_SPY_KIT
};
