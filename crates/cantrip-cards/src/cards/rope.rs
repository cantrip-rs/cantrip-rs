use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::{combat::one_blocker_at_most, effects, statics};
use cantrip_oracle as oracle;

/// "Equipped creature gets +1/+2, has reach, and can't be blocked by more than one creature." (CR 613.4c, 702.17, 509.1b): the first two
/// are layer effects on the equipped creature, the last a rule of the Equipment's (CR 113.1a), as for an Aura. "{2}, Sacrifice this
/// Equipment: Draw a card." (CR 602.2, 701.21): sacrificing it is part of the cost, so the ability resolves without it. It is a Clue
/// by subtype only (CR 111.10f); the Clue's own ability is the one printed here.
pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}").and(SACRIFICE_THIS), |g, item| effects::draw(g, item, 1), "{2}, Sacrifice this Equipment: Draw a card."),
        equip(mana("{3}"), "Equip {3}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, statics::plus_pt::<1, 2>, "Equipped creature gets +1/+2."),
        static_ability(Layer::Ability, attached_object, statics::have_reach, "Equipped creature has reach."),
    ],
    blocker_limits: &[one_blocker_at_most],
    ..oracle::ROPE
};
