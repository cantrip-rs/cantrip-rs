use cantrip_api::*;
use cantrip_mechanics::{effects, lands, statics, targets};
use cantrip_oracle as oracle;

/// "Thrull creatures get +1/+1." (CR 604.1, 613.1f).
fn thrull_creatures(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::all_of_subtype::<{ lands::sub(Subtype::Thrull) }>(q, source, affected, so_far) && so_far.is(CardType::Creature)
}

/// "Target Thrull" (CR 115.1).
fn target_thrull(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Thrull])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP,
            effects::gain_control_of_target_while_you_control_source,
            "{T}: Gain control of target Thrull for as long as you control this creature.",
        )
        .targets(&[target(target_thrull)]),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            thrull_creatures,
            statics::plus_pt::<1, 1>,
            "Thrull creatures get +1/+1.",
        ),
    ],
    ..oracle::THRULL_CHAMPION
};
