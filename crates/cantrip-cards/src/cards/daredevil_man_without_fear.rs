use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::library::exile_top_play_this_turn;
use cantrip_mechanics::{pump, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ExileTop", "exile the top card of your library?", "Chosen as Daredevil's trigger resolves", "Exile it", "Don't");

/// "You may exile the top card of your library. If that card is a Hero card, Daredevil gets +2/+1 until end of turn. You may
/// play that card this turn." (CR 305.1, 601.3, 611.2): the card is a Hero card by the subtype it has in exile (CR 205.3).
fn exile_the_top_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let exiled = exile_top_play_this_turn(g, item.controller, 1);
    if exiled.iter().any(|&card| g.chars(card).has_subtype(g.words(item.object).subtype(Subtype::Hero))) {
        pump(g, item.object, item.object, 2, 1);
    }
    Ok(())
}

/// "You may look at the top card of your library any time" (CR 401.2).
pub static CARD: CardDef = CardDef {
    play_rules: &[PlayRule::LookAtLibraryTop],
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        exile_the_top_card,
        "Whenever you attack, you may exile the top card of your library. If that card is a Hero card, Daredevil gets +2/+1 until end of turn. You may play that card this turn.",
    )],
    ..oracle::DAREDEVIL_MAN_WITHOUT_FEAR
};
