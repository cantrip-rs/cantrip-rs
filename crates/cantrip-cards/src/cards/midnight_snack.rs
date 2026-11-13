use cantrip_api::*;
use cantrip_mechanics::{raid::you_attacked_this_turn, targets, tokens, triggers::YOUR_END_STEP};
use cantrip_oracle as oracle;

/// "Target opponent loses X life, where X is the amount of life you gained this turn" (CR 119.3, 608.2h): X is read from the
/// event history as the ability resolves, the life gained on this turn only and all of it, however many events it came in. An
/// opponent no longer a legal target loses nothing (CR 608.2b).
fn opponent_loses_life_gained_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Target::Player(opponent) = item.target(0)
    {
        let x = g.history().life_gained(item.controller) as i32;
        g.lose_life(opponent, x);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            YOUR_END_STEP,
            tokens::create_a_food_token,
            "Raid \u{2014} At the beginning of your end step, if you attacked this turn, create a Food token.",
        )
        .condition(you_attacked_this_turn),
        ability(
            cost("{2}{B}").and(SACRIFICE_THIS),
            opponent_loses_life_gained_this_turn,
            "{2}{B}, Sacrifice this enchantment: Target opponent loses X life, where X is the amount of life you gained this turn.",
        )
        .targets(&[targets::OPPONENT]),
    ],
    ..oracle::MIDNIGHT_SNACK
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
