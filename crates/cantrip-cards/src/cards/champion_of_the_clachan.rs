use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::{other_of_subtype_you_control, plus_pt};
use cantrip_mechanics::{behold, effects, triggers::THIS_LEAVES};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, behold a Kithkin and exile it" (CR 601.2b, 701.4a): the Kithkin is a
/// permanent or a card in hand, and is exiled as the cost is paid; it is exiled with the creature the spell becomes (CR
/// 607.2q), which returns it when it leaves the battlefield. A spell that is countered leaves it exiled. "Other Kithkin you
/// control get +1/+1" (CR 613.4c, layer 7c).
pub static CARD: CardDef = CardDef {
    additional_cost: Some(behold::cost_and_exile(behold::kithkin, "Kithkin")),
    statics: &[static_ability(
        Layer::PtModify,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Kithkin) }>,
        plus_pt::<1, 1>,
        "Other Kithkin you control get +1/+1.",
    )],
    abilities: &[triggered(THIS_LEAVES, effects::return_exiled_to_hand, "When this creature leaves the battlefield, return the exiled card to its owner's hand.")],
    ..oracle::CHAMPION_OF_THE_CLACHAN
};
