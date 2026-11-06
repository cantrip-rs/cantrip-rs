use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Attacking creatures get -1/-0." (CR 604.1).
fn attacking_creatures_get_1_0_affects(
    q: &dyn LayerQuery,
    _: ObjectId,
    affected: ObjectId,
    so_far: &Chars,
) -> bool {
    q.state().attackers.iter().any(|a| a.id == affected)
        && (*so_far).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            attacking_creatures_get_1_0_affects,
            cantrip_mechanics::statics::plus_pt_saturating::<-1, 0>,
            "Attacking creatures get -1/-0.",
        ),
    ],
    ..oracle::WEAKSTONE
};
