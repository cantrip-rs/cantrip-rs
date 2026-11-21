use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{earthbend, effects, learn, targets};
use cantrip_oracle as oracle;

/// Mode 2: "Target player scries 3" (CR 701.22a): the targeted player, if still legal, looks at their own library.
fn target_player_scries_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    cantrip_mechanics::scry(g, player, item.object, 3)
}

/// "X is the number of Lesson cards in your graveyard" (CR 205.3k): counted as the ability is put on the stack, when its modes are
/// chosen (CR 603.3c, 700.2b).
fn lessons_in_your_graveyard(q: &dyn Query, asker: &Asker) -> u32 {
    learn::lesson_cards_in_graveyard(q, asker.controller) as u32
}

/// "Choose up to X —" (CR 700.2, 603.3c): each mode at most once, so X past three adds nothing; with X = 0 no mode is chosen and the
/// ability is removed from the stack.
static MODAL: ModalDef = modes(&[
    mode(effects::three_plus_one_counters_on_this, "Put three +1/+1 counters on Bumi."),
    mode(target_player_scries_3, "Target player scries 3.").targets(&[targets::PLAYER]),
    mode(earthbend::effect::<3>, "Earthbend 3.").targets(&[earthbend::TARGET]),
])
.up_to_that_many();

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When Bumi enters, choose up to X, where X is the number of Lesson cards in your graveyard \u{2014} Put three +1/+1 counters on Bumi; or target player scries 3; or earthbend 3.",
    )
    .counted_by(lessons_in_your_graveyard)],
    delayed: &[earthbend::RETURN],
    ..oracle::BUMI_KING_OF_THREE_TRIALS
};
