use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{effects, your_turn};
use cantrip_oracle as oracle;

/// "Creatures your opponents control get -1/-1" (CR 613.4c, layer 7c): every creature, as the earlier layers made it, under the
/// control of an opponent of this permanent's controller.
fn opponents_creatures(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let s = q.state();
    so_far.is(CardType::Creature) && s.opponents(s.obj(source).controller).any(|o| o == s.obj(affected).controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(life(3), effects::this_connives, "Mental Organism \u{2014} Pay 3 life: M.O.D.O.K. connives. Activate only during your turn.").condition(your_turn),
    ],
    statics: &[static_ability(Layer::PtModify, opponents_creatures, plus_pt::<-1, -1>, "Designed Only for Killing \u{2014} Creatures your opponents control get -1/-1.")],
    ..oracle::M_O_D_O_K
};
