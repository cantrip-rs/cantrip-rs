use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{attach::attached_object, attach::enchanted_creature_gains_until_end_of_turn, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

fn hexproof(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    enchanted_creature_gains_until_end_of_turn(g, item, Keyword::Hexproof);
    Ok(())
}

/// Flash is a core keyword.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_YOU_CONTROL),
    abilities: &[triggered(THIS_ENTERS, hexproof, "When this Aura enters, enchanted creature gains hexproof until end of turn.")],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 2>, "Enchanted creature gets +1/+2.")],
    ..oracle::AQUITECTS_DEFENSES
};
