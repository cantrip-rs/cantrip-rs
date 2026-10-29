use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets};
use cantrip_oracle as oracle;

/// "Exile the top two cards of your library. You may play those cards until your next end step": a permission to play, so a
/// land may be the turn's land drop (CR 305.1), ending as the controller's next end step begins (CR 500.4).
fn exile_and_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::exile_top_play_until_your_next_end_step(g, item.controller, 2);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(exile_and_play, "Exile the top two cards of your library. You may play those cards until your next end step."),
    mode(effects::targets_get::<2, 0>, "One or two target creatures each get +2/+0 until end of turn.").targets(&[targets::CREATURE.between(1, 2)]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::OPERA_LOVE_SONG };
