use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, attach::enchanted_creature_gains_until_end_of_turn, statics::plus_pt, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

fn first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    enchanted_creature_gains_until_end_of_turn(g, item, Keyword::FirstStrike);
    Ok(())
}

/// Flash is a core keyword.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        THIS_ENTERS,
        first_strike,
        "When this Aura enters, enchanted creature gains first strike until end of turn.",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        attached_object,
        plus_pt::<2, 0>,
        "Enchanted creature gets +2/+0.",
    )],
    ..oracle::CAPTAINS_TUTELAGE
};
