use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_DRAW_YOUR_SECOND_CARD;
use cantrip_mechanics::{effects, lands, statics, targets};
use cantrip_oracle as oracle;

/// "Target Detective" (CR 115.1, 205.3m): a Detective on the battlefield, whoever controls it, the type read through the words of this
/// card's text (CR 612.2).
fn detective(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Detective])
}

/// "Other Detectives you control get +1/+1" (CR 613.4c, layer 7c): every other Detective its controller controls as the layers have
/// it so far, never Private Eye itself.
/// "Whenever you draw your second card each turn, target Detective can't be blocked this turn" (CR 121.1, 509.1b, 611.2): the target is
/// chosen as the ability goes on the stack, and an effect that lasts until end of turn on whatever Detective it was, any controller's.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Detective) }>,
        statics::plus_pt::<1, 1>,
        "Other Detectives you control get +1/+1.",
    )],
    abilities: &[triggered(
        YOU_DRAW_YOUR_SECOND_CARD,
        effects::target_creature_cant_be_blocked_this_turn,
        "Whenever you draw your second card each turn, target Detective can't be blocked this turn.",
    )
    .targets(&[target(detective)])],
    ..oracle::PRIVATE_EYE
};
