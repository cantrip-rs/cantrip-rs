use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "Choose one —" (CR 700.2). The second mode's "Exile target creature you control. Return that card to the battlefield
/// under its owner's control at the beginning of the next end step" (CR 701.13a, 603.7c, 400.7) is the shared slot-0 body:
/// the delayed ability is about the card it became in exile.
static MODAL: ModalDef = modes(&[
    mode(effects::surveil_2_then_draw_a_card, "Power to Engines \u{2014} Surveil 2, then draw a card."),
    mode(delayed::exile_target_then_return_at_next_end_step_0, "Power to Life Support \u{2014} Exile target creature you control. Return that card to the battlefield under its owner's control at the beginning of the next end step.")
        .targets(&[targets::CREATURE_YOU_CONTROL]),
]);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    delayed: &[delayed_trigger(
        delayed::return_subject_to_battlefield,
        "Return that card to the battlefield under its owner's control at the beginning of the next end step.",
    )],
    ..oracle::TEST_OF_PRIORITIES
};
