use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets, triggers};
use cantrip_oracle as oracle;

/// "Enchant creature or Vehicle" (CR 303.4a): a creature, or a Vehicle that is not a creature just now (CR 301.7).
/// "When this Aura enters, untap enchanted permanent." (CR 701.26a): the permanent the Aura is attached to as the ability
/// resolves, or, if the Aura has left the battlefield by then, the one it was attached to as it left (CR 608.2h); an Aura
/// that was never attached has none (CR 303.4g).
fn untap_enchanted(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = g.state().obj(item.object).attached_to else { return Ok(()) };
    instruction(g, |g| g.untap(permanent));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_OR_VEHICLE),
    abilities: &[triggered(triggers::THIS_ENTERS, untap_enchanted, "When this Aura enters, untap enchanted permanent.")],
    statics: &[
        static_ability(Layer::Ability, attach::attached_object, statics::have_reach, "Enchanted permanent gets +1/+2 and has reach."),
        static_ability(Layer::PtModify, attach::attached_object, statics::plus_pt::<1, 2>, "Enchanted permanent gets +1/+2 and has reach.").continues(),
    ],
    ..oracle::SILKEN_STRENGTH
};
