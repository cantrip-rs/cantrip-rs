use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, discover, statics, targets, triggers};
use cantrip_oracle as oracle;

/// "When this Aura enters, discover 3." (CR 701.57a).
fn discover_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discover::discover(g, item.controller, 3)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_YOU_CONTROL),
    abilities: &[triggered(triggers::THIS_ENTERS, discover_3, "When this Aura enters, discover 3.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, statics::plus_pt::<1, 1>, "Enchanted creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object, statics::have_trample, "Enchanted creature has trample."),
    ],
    ..oracle::ETALIS_FAVOR
};
