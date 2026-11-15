use cantrip_api::*;
use cantrip_mechanics::statics::your_opponents_creatures;
use cantrip_mechanics::triggers::ward_discard;
use cantrip_oracle as oracle;

const WARD: &str = "Ward\u{2014}Discard a card. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player discards a card.)";

/// "Creatures your opponents control have base toughness 1" (CR 613.4b, layer 7b): only the toughness is set, so power,
/// counters and later pumps (layer 7c) are untouched.
fn base_toughness_1(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.toughness = Some(1);
}

pub static CARD: CardDef = CardDef {
    abilities: &[ward_discard(WARD)],
    statics: &[static_ability(Layer::PtSet, your_opponents_creatures, base_toughness_1, "Creatures your opponents control have base toughness 1.")],
    ..oracle::MAHA_ITS_FEATHERS_NIGHT
};
