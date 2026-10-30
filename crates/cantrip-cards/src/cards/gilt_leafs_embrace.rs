use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{attach::attached_object, effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this Aura enters, enchanted creature gains trample and indestructible until end of turn" (CR 611.2a, 702.19, 702.12).
/// The enchanted creature is the one the Aura is attached to as the ability resolves, or, if the Aura has left the battlefield
/// by then, the one it was attached to as it left (CR 608.2h); an effect on a creature that has left does nothing (CR 400.7).
fn trample_and_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(enchanted) = g.obj(item.object).attached_to else { return Ok(()) };
    instruction(g, |g| {
        effects::gain_until_end_of_turn(g, item.object, enchanted, Keyword::Trample);
        effects::gain_until_end_of_turn(g, item.object, enchanted, Keyword::Indestructible);
    });
    Ok(())
}

/// Flash is a core keyword.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, trample_and_indestructible, "When this Aura enters, enchanted creature gains trample and indestructible until end of turn.")],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 0>, "Enchanted creature gets +2/+0.")],
    ..oracle::GILT_LEAFS_EMBRACE
};
