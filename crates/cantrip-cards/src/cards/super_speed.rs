use cantrip_api::*;
use cantrip_mechanics::statics::{have_haste, plus_pt};
use cantrip_mechanics::{attach::attached_object, attach::enchanted_creature_gains_until_end_of_turn, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

fn first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    enchanted_creature_gains_until_end_of_turn(g, item, Keyword::FirstStrike);
    Ok(())
}

/// Flash is a core keyword.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, first_strike, "When this Aura enters, enchanted creature gains first strike until end of turn.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Enchanted creature gets +1/+0."),
        static_ability(Layer::Ability, attached_object, have_haste, "Enchanted creature has haste."),
    ],
    ..oracle::SUPER_SPEED
};
