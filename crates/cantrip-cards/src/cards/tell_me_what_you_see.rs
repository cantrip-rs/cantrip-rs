use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

const DRAW: Question = Question::new(
    "DrawForYou",
    "have that player draw a card?",
    "Otherwise you lose 4 life and that player surveils 1",
    "Draw",
    "Don't",
);

/// "Target opponent may have you draw a card. If they don't, they lose 4 life and you surveil 1."
/// The opponent chooses; the life loss and surveil happen only on declining (CR 118.12, 119.3, 701.25).
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    if effects::ask_may(g, opponent, item.object, Purpose::Ask(&DRAW))? {
        g.draw(item.controller, 1);
    } else {
        g.lose_life(opponent, 4);
        g.end_sentence();
        effects::surveil(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, enters, "When this enchantment enters, target opponent may have you draw a card. If they don't, they lose 4 life and you surveil 1.").targets(&[targets::OPPONENT]),
        triggered(triggers::YOU_FACE_A_DILEMMA, effects::may_return_this_to_hand, "Whenever you face a dilemma, you may return this enchantment to its owner's hand."),
    ],
    ..oracle::TELL_ME_WHAT_YOU_SEE
};
