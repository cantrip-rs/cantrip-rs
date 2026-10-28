use cantrip_api::*;
use cantrip_mechanics::statics::{have_haste, plus_pt};
use cantrip_mechanics::{attach::attached_object, effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this Aura enters, enchanted creature gains first strike until end of turn" (CR 611.2a, 702.7). The enchanted creature is
/// the one the Aura is attached to as the ability resolves, or, if the Aura has left the battlefield by then, the one it was
/// attached to as it left (CR 608.2h); an effect on a creature that has left does nothing (CR 400.7).
fn first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(enchanted) = g.obj(item.object).attached_to {
        effects::gain_until_end_of_turn(g, item.object, enchanted, Keyword::FirstStrike);
    }
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
