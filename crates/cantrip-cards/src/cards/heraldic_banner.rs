use cantrip_api::*;
use cantrip_mechanics::choose_color_for;
use cantrip_mechanics::lands::TAP_FOR_THE_CHOSEN_COLOR;
use cantrip_mechanics::statics::{plus_pt, your_creatures};
use cantrip_oracle as oracle;

/// "As this artifact enters, choose a color" (CR 614.12a), kept on the permanent (CR 607.2d).
fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.chosen_color = Some(choose_color_for(g, entering.controller, entering.object, ColorUse::Benefit)?);
    Ok(())
}

/// "Creatures you control of the chosen color" (CR 613.4c): creatures under the Banner's controller's control that are the
/// chosen color after the color layer (CR 613.1e), a multicolored one included, a colorless one never.
fn your_creatures_of_the_chosen_color(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    q.state().chosen_color(q.linked_to(source)).is_some_and(|color| so_far.colors.has(color)) && your_creatures(q, source, affected, so_far)
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters),
    abilities: &[TAP_FOR_THE_CHOSEN_COLOR],
    statics: &[static_ability(Layer::PtModify, your_creatures_of_the_chosen_color, plus_pt::<1, 0>, "Creatures you control of the chosen color get +1/+0.")],
    ..oracle::HERALDIC_BANNER
};
