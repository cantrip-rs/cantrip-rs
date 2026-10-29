use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "This creature gets +2/+0 for each other Goblin you control." (CR 613.4c, layer 7c): the other permanents its controller
/// controls that are Goblins as the layers have them so far, the type read through the words of this card's text (CR 612.2),
/// counted again whenever characteristics are computed.
fn plus_two_for_each_other_goblin(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = q.state();
    let you = state.obj(source).controller;
    let goblin = q.words(source).subtype(Subtype::Goblin);
    let others = state.battlefield().iter().filter(|&&id| id != source && state.obj(id).controller == you && q.chars(id).has_subtype(goblin)).count() as i16;
    chars.power = chars.power.map(|p| p + 2 * others);
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, statics::itself, plus_two_for_each_other_goblin, "This creature gets +2/+0 for each other Goblin you control.")],
    ..oracle::BONECLUB_BERSERKER
};
