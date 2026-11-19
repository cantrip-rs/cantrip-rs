use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::{effects, history, tokens, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of each end step, if a creature died this turn, create a Food token." (CR 603.4, 700.4, 111.10b): an
/// intervening "if", checked as the end step begins and again as the ability resolves, so it makes a Food in every end step of a
/// turn in which a creature died, whoever's turn it is.
///
/// "{2}{B}, Sacrifice another creature or artifact: Old Flitterfang gets +2/+2 until end of turn." (CR 118.3, 701.21, 611.2a)
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::EACH_END_STEP,
            tokens::create_a_food_token,
            "At the beginning of each end step, if a creature died this turn, create a Food token.",
        )
        .condition(history::a_creature_died_this_turn),
        ability(
            cost("{2}{B}").and(sacrifice(1, filters::another_creature_or_artifact)),
            effects::this_gets::<2, 2>,
            "{2}{B}, Sacrifice another creature or artifact: Old Flitterfang gets +2/+2 until end of turn.",
        ),
    ],
    ..oracle::OLD_FLITTERFANG
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
