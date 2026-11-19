use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        effects::draw_a_card_and_lose_1_life,
        "Descend 4 \u{2014} Whenever this creature attacks, if there are four or more permanent cards in your graveyard, you draw a card and you lose 1 life.",
    )
    .condition(targets::you_have_permanent_cards_in_graveyard::<4>)],
    ..oracle::STINGING_CAVE_CRAWLER
};
