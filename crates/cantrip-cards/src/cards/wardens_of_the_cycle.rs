use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(|g, item| effects::gain_life(g, item, 2), "You gain 2 life."),
    mode(effects::draw_a_card_and_lose_1_life, "You draw a card and you lose 1 life."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        YOUR_END_STEP,
        &MODAL,
        "Morbid — At the beginning of your end step, if a creature died this turn, choose one — You gain 2 life; or You draw a card and you lose 1 life.",
    )
    .condition(history::a_creature_died_this_turn)],
    ..oracle::WARDENS_OF_THE_CYCLE
};
