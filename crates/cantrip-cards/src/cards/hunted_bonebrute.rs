use cantrip_api::*;
use cantrip_mechanics::{disguise, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, target opponent creates two 1/1 white Dog creature tokens." (CR 111.2, 608.2b): the tokens are made
/// under that opponent's control, together; nothing happens if they are no longer a legal target as the ability resolves.
fn target_opponent_creates_two_dogs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    tokens::create(g, opponent, &tokens::DOG, 2)?;
    Ok(())
}

/// "Each opponent loses 3 life." (CR 119.3): every opponent still in the game loses it together.
fn each_opponent_loses_3_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.each_opponent_loses_life(item.controller, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, target_opponent_creates_two_dogs, "When this creature enters, target opponent creates two 1/1 white Dog creature tokens.")
            .targets(&[targets::OPPONENT]),
        triggered(triggers::THIS_DIES, each_opponent_loses_3_life, "When this creature dies, each opponent loses 3 life."),
    ],
    ways: &[disguise::way(Cost::mana(mana("{1}{B}")), "Disguise {1}{B}")],
    ..oracle::HUNTED_BONEBRUTE
};

pub static EXTRA: &[&CardDef] = &[&tokens::DOG];
