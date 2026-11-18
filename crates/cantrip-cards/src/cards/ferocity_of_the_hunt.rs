use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::statics::{have_deathtouch, plus_pt};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return that card to the battlefield tapped under its owner's control": every card the dead permanent became, a merged one's
/// together (CR 730.3c), each put onto the battlefield tapped (CR 110.5b) under its owner's control (CR 110.2a).
fn return_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    effects::return_what_it_became(g, creature, true, &[])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        cantrip_mechanics::triggers::ATTACHED_CREATURE_DIES_ABOUT_IT,
        return_tapped,
        "When enchanted creature dies, return that card to the battlefield tapped under its owner's control.",
    )],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Enchanted creature gets +1/+0."),
        static_ability(Layer::Ability, attached_object, have_deathtouch, "Enchanted creature has deathtouch."),
    ],
    ..oracle::FEROCITY_OF_THE_HUNT
};
