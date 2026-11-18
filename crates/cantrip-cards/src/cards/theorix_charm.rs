use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::counter_first_target_unless_pays::<2>,"Counter target noncreature spell unless its controller pays {2}.").targets(&[targets::NONCREATURE_SPELL]),
    mode(effects::target_creature_gets::<-2, -2>, "Target creature gets -2/-2 until end of turn.").targets(&[targets::CREATURE]),
    mode(effects::you_mill_then_draw::<3, 1>, "Mill three cards, then draw a card."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::THEORIX_CHARM };
