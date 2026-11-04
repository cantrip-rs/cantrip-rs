use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "During your turn, creatures you control get +2/+0." (CR 604.1).
fn your_turn(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    q.controller(source) == q.state().turn.active && statics::your_creatures(q, source, affected, chars)
}

/// "During turns other than yours, creatures you control get -0/-2." (CR 604.1).
fn other_turn(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    q.controller(source) != q.state().turn.active && statics::your_creatures(q, source, affected, chars)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::PtModify, your_turn, statics::plus_pt_saturating::<2, 0>, "During your turn, creatures you control get +2/+0."),
        static_ability(Layer::PtModify, other_turn, statics::plus_pt_saturating::<0, -2>, "During turns other than yours, creatures you control get -0/-2."),
    ],
    ..oracle::VIBRATING_SPHERE
};
