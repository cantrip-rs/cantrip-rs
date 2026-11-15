use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this Equipment enters, create a 1/1 white Glimmer enchantment creature token" (CR 111.1): one, for the ability's controller.
fn create_glimmer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::GLIMMER, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_glimmer, "When this Equipment enters, create a 1/1 white Glimmer enchantment creature token."),
        equip(mana("{1}"), "Equip {1}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    ..oracle::GLIMMERLIGHT
};

pub static EXTRA: &[&CardDef] = &[&tokens::GLIMMER];
