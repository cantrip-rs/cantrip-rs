use cantrip_api::*;
use cantrip_mechanics::{raid::you_attacked_this_turn, tokens, triggers::YOUR_END_STEP};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        tokens::create_goblins::<1>,
        "Raid \u{2014} At the beginning of your end step, if you attacked this turn, create a 1/1 red Goblin creature token.",
    )
    .condition(you_attacked_this_turn)],
    ..oracle::SEARSLICER_GOBLIN
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN];
