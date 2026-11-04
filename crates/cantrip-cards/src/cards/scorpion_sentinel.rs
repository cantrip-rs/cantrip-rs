use cantrip_api::*;
use cantrip_mechanics::{lands, statics::plus_pt};
use cantrip_oracle as oracle;

/// "As long as you control seven or more lands, this creature gets +3/+0" (CR 613.4c, 604.1): itself, while its controller
/// controls seven or more lands, judged live through the layers (CR 611.3a) - a land is one as the layers made it, a land the
/// opponent controls does not count (`lands::count_you_control`).
fn itself_with_seven_or_more_lands(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    affected == source && lands::count_you_control(q, source, &|c| c.is(CardType::Land)) >= 7
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, itself_with_seven_or_more_lands, plus_pt::<3, 0>, "As long as you control seven or more lands, this creature gets +3/+0.")],
    ..oracle::SCORPION_SENTINEL
};
