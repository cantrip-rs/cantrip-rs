use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{attach, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this Aura enters, suspect enchanted creature" (CR 701.60a, 303.4): the creature the Aura is attached to as the
/// trigger resolves, or, if the Aura has left the battlefield by then, the one it was attached to as it left (CR 608.2h).
/// A creature that has left the battlefield is a new object nothing reaches (CR 400.7).
fn suspect_enchanted_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(enchanted) = g.obj(item.object).attached_to {
        g.suspect(enchanted);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, suspect_enchanted_creature, "When this Aura enters, suspect enchanted creature."),
        // It functions only in its owner's graveyard (CR 113.6m).
        ability(cost("{2}{R}"), effects::return_this_to_hand, "{2}{R}: Return this card from your graveyard to your hand.").from(ZoneKind::Graveyard),
    ],
    statics: &[static_ability(Layer::PtModify, attach::attached_object, plus_pt::<1, 1>, "Enchanted creature gets +1/+1.")],
    enchant: Some(targets::CREATURE),
    ..oracle::CONVENIENT_TARGET
};
