use cantrip_api::*;
use cantrip_mechanics::statics::{have_trample, plus_pt, your_creatures};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Creatures you control ... as long as you gained life this turn" (CR 611.3a, 119.3): the creatures of the Striker's controller
/// while that player has gained life this turn. The layers ask again whenever characteristics are computed, so it turns on with the
/// gain, covers creatures that arrive later, and off as the turn ends.
fn your_creatures_while_you_gained_life(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    your_creatures(q, source, affected, so_far) && q.history().life_gained(q.state().obj(source).controller) > 0
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::ward::<1>(
        "Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)",
    )],
    statics: &[
        static_ability(
            Layer::PtModify,
            your_creatures_while_you_gained_life,
            plus_pt::<1, 0>,
            "Infusion \u{2014} Creatures you control get +1/+0 as long as you gained life this turn.",
        ),
        static_ability(
            Layer::Ability,
            your_creatures_while_you_gained_life,
            have_trample,
            "Infusion \u{2014} Creatures you control have trample as long as you gained life this turn.",
        ),
    ],
    ..oracle::THORNFIST_STRIKER
};
