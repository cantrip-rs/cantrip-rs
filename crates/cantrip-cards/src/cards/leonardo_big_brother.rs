use cantrip_api::*;
use cantrip_mechanics::sneak;
use cantrip_oracle as oracle;

/// "Leonardo gets +1/+0 for each other creature you control" (CR 613.4c, layer 7c): the creatures as the earlier layers made
/// them, under the control of Leonardo's controller, not Leonardo himself.
fn plus_power_per_other_creature(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = view.state();
    let you = state.obj(source).controller;
    let n = state.battlefield().iter().filter(|&&id| id != source && state.obj(id).controller == you && view.chars(id).is(CardType::Creature)).count() as i16;
    chars.power = chars.power.map(|v| v + n);
}

pub static CARD: CardDef = CardDef {
    ways: &[sneak::way(mana("{W}"), "Sneak {W}")],
    statics: &[static_ability(Layer::PtModify, cantrip_mechanics::statics::itself, plus_power_per_other_creature, "Leonardo gets +1/+0 for each other creature you control.")],
    ..oracle::LEONARDO_BIG_BROTHER
};
