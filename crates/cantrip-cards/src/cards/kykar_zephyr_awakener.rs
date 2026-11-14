use cantrip_api::*;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST;
use cantrip_mechanics::{delayed, targets, tokens};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(delayed::exile_target_then_return_at_next_end_step_0, "Exile another target creature you control. Return that card to the battlefield under its owner's control at the beginning of the next end step.").targets(&[targets::OTHER_CREATURE_YOU_CONTROL]),
    mode(tokens::create_a_white_flying_spirit, "Create a 1/1 white Spirit creature token with flying."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(NONCREATURE_SPELL_YOU_CAST, &MODAL, "Whenever you cast a noncreature spell, choose one \u{2014}\n\u{2022} Exile another target creature you control. Return that card to the battlefield under its owner's control at the beginning of the next end step.\n\u{2022} Create a 1/1 white Spirit creature token with flying.")],
    delayed: &[delayed_trigger(delayed::return_subject_to_battlefield, "Return that card to the battlefield under its owner's control at the beginning of the next end step.")],
    ..oracle::KYKAR_ZEPHYR_AWAKENER
};

pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_FLYING_SPIRIT];
