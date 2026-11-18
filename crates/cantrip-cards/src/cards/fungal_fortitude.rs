use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Return it to the battlefield tapped under its owner's control": every card the dead permanent became, a merged one's
/// together (CR 730.3c), each put onto the battlefield tapped (CR 110.5b) under its owner's control (CR 110.2a).
fn return_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    effects::return_what_it_became(g, creature, true, &[])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        triggers::ATTACHED_CREATURE_DIES_ABOUT_IT,
        return_tapped,
        "When enchanted creature dies, return it to the battlefield tapped under its owner's control.",
    )],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 0>, "Enchanted creature gets +2/+0.")],
    ..oracle::FUNGAL_FORTITUDE
};
