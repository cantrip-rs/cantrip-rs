use cantrip_api::*;
use cantrip_mechanics::statics::{have_reach, plus_pt};
use cantrip_mechanics::{attach, effects, targets, triggers::THIS_DIES};
use cantrip_oracle as oracle;

/// Flash is a core keyword. "Enchanted creature gets +3/+2 and has reach" (CR 613.4c, 702.17); "When this Aura is put into a graveyard from the
/// battlefield, draw a card" (CR 603.6c, 700.4), whether the creature died, the Aura was destroyed or it fell off as a state-based action.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        THIS_DIES,
        |g, item| effects::draw(g, item, 1),
        "When this Aura is put into a graveyard from the battlefield, draw a card.",
    )],
    statics: &[
        static_ability(Layer::Ability, attach::attached_object, have_reach, "Enchanted creature gets +3/+2 and has reach."),
        static_ability(Layer::PtModify, attach::attached_object, plus_pt::<3, 2>, "Enchanted creature gets +3/+2 and has reach.").continues(),
    ],
    ..oracle::REACH_FOR_THE_SKY
};
