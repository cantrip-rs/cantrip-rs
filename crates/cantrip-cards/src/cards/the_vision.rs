use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(|g, item| effects::this_gains_until_end_of_turn(g, item, Keyword::DoubleStrike), "Solar Beam \u{2014} The Vision gains double strike until end of turn."),
    mode(|g, item| effects::this_gains_until_end_of_turn(g, item, Keyword::Indestructible), "Density Control \u{2014} The Vision gains indestructible until end of turn."),
    mode(|g, item| effects::draw(g, item, 1), "Technopathy \u{2014} Draw a card."),
])
.allowed(history::mode_not_chosen_this_turn);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::NONCREATURE_SPELL_YOU_CAST,
        &MODAL,
        "Whenever you cast a noncreature spell, choose one that hasn't been chosen this turn \u{2014} Solar Beam: double strike until end of turn; or Density Control: indestructible until end of turn; or Technopathy: draw a card.",
    )],
    ..oracle::THE_VISION
};
