use cantrip_api::*;
use cantrip_mechanics::statics::{have_flying, plus_pt};
use cantrip_mechanics::{attach, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this Aura enters, enchanted creature gains hexproof until end of turn." (CR 702.11b). Flash is a core keyword.
fn hexproof(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    attach::enchanted_creature_gains_until_end_of_turn(g, item, Keyword::Hexproof);
    Ok(())
}

/// "Enchant creature" (CR 303.4) with "Enchanted creature gets +1/+0 and has flying." (CR 613.4c, 613.1f).
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, hexproof, "When this Aura enters, enchanted creature gains hexproof until end of turn.")],
    statics: &[
        static_ability(Layer::Ability, attach::attached_object, have_flying, "Enchanted creature gets +1/+0 and has flying."),
        static_ability(Layer::PtModify, attach::attached_object, plus_pt::<1, 0>, "Enchanted creature gets +1/+0 and has flying.").continues(),
    ],
    ..oracle::FAE_FLIGHT
};
