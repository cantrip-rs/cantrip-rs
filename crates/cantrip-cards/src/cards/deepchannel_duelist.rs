use cantrip_api::*;
use cantrip_mechanics::statics::{other_of_subtype_you_control, plus_pt};
use cantrip_mechanics::{lands, targets, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your end step, untap target Merfolk you control."
fn untap_merfolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(merfolk) = targets::object(g, item, 0) else { return Ok(()) };
    g.untap(merfolk);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_END_STEP, untap_merfolk, "At the beginning of your end step, untap target Merfolk you control.")
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Merfolk) }>)])],
    statics: &[static_ability(
        Layer::PtModify,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Merfolk) }>,
        plus_pt::<1, 1>,
        "Other Merfolk you control get +1/+1.",
    )],
    ..oracle::DEEPCHANNEL_DUELIST
};
