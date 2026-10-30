use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "This creature gets +1/+1 for each creature you control and each creature card in your graveyard" (CR 613.4c, layer 7c): the
/// creatures are counted as the earlier layers left them, this one among them, and a creature card in the graveyard by the
/// types the card has there.
fn plus_one_per_creature_and_creature_card(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = view.state();
    let you = state.obj(source).controller;
    let on_battlefield = state.battlefield().iter().filter(|&&id| state.obj(id).controller == you && view.chars(id).is(CardType::Creature)).count();
    let in_graveyard = state.zone(Zone::graveyard(you)).iter().filter(|&&id| view.chars(id).is(CardType::Creature)).count();
    let n = (on_battlefield + in_graveyard) as i16;
    chars.power = chars.power.map(|v| v + n);
    chars.toughness = chars.toughness.map(|v| v + n);
}

/// Trample is a core keyword.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::itself,
        plus_one_per_creature_and_creature_card,
        "This creature gets +1/+1 for each creature you control and each creature card in your graveyard.",
    )],
    ..oracle::MOON_VIGIL_ADHERENTS
};
