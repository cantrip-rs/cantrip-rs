use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::triggers::YOU_DISCARD;
use cantrip_oracle as oracle;

const EXILE: Question = Question::new(
    "Exile",
    "exile that card from your graveyard?",
    "If you do, until the end of your next turn you may play it",
    "Exile it",
    "Leave it",
);

/// "You may exile that card from your graveyard. If you do, until the end of your next turn, you may play that card." (CR 701.9a,
/// 400.7, 601.3, 305.1): the trigger is about the card as it is in the graveyard; once it has left (a new object, CR 400.7) there
/// is nothing to exile. A permission to play, not only to cast, so a land may be played as the turn's land drop, for the card the
/// exile made.
fn exile_to_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(card) = item.subject else { return Ok(()) };
    if !is_live(g, card) || g.obj(card).zone != Zone::graveyard(item.controller) {
        return Ok(());
    }
    if ask_may(g, item.controller, item.object, Purpose::Ask(&EXILE))? {
        let exiled = g.exile(card);
        g.allow_play(exiled, item.controller, PlayGrant { lands: true, expiry: Expiry::EndOfYourNextTurn, ..PlayGrant::CAST });
    }
    Ok(())
}

/// Flying is a core keyword read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_DISCARD,
        exile_to_play,
        "Whenever you discard a card, you may exile that card from your graveyard. If you do, until the end of your next turn, you may play that card.",
    )],
    ..oracle::MOONSTONE_HARSH_MISTRESS
};
