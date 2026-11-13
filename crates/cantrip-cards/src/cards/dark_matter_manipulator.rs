use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::statics::itself;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "This creature gets +2/+0 for every seven cards in your graveyard" (CR 613.4c): +2 power for each full seven cards, counted
/// afresh in the graveyard of its controller every time the layers are applied; tokens are not cards and are never there.
fn plus_two_per_seven(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = view.state();
    let sevens = state.zone(Zone::graveyard(state.obj(source).controller)).len() / 7;
    chars.power = chars.power.map(|p| p + 2 * sevens as i32);
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::you_mill::<3>, "When this creature enters, mill three cards.")],
    statics: &[static_ability(Layer::PtModify, itself, plus_two_per_seven, "This creature gets +2/+0 for every seven cards in your graveyard.")],
    ..oracle::DARK_MATTER_MANIPULATOR
};
