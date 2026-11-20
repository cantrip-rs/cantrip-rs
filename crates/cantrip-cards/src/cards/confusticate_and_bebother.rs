use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::counter_first_target_unless_pays::<4>,"Counter target spell unless its controller pays {4}.").targets(&[targets::SPELL]),
    mode(effects::draw_then_discard::<2, 1>, "Draw two cards, then discard a card."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::CONFUSTICATE_AND_BEBOTHER };
