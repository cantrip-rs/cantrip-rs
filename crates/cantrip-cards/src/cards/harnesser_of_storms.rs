use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::library::exile_top_play_this_turn;
use cantrip_mechanics::{lands, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ExileTop", "exile the top card of your library?", "Chosen as Harnesser of Storms' trigger resolves  ·  you may play it this turn", "Exile it", "Don't");

/// "You may exile the top card of your library. Until end of turn, you may play that card." (CR 305.1, 601.3, 611.2): the card is
/// played, not only cast, so a land among them may be the turn's land drop. With an empty library nothing is exiled.
fn exile_the_top_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        exile_top_play_this_turn(g, item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::noncreature_or_subtype_spell_you_cast::<{ lands::sub(Subtype::Otter) }>),
        exile_the_top_card,
        "Whenever you cast a noncreature or Otter spell, you may exile the top card of your library. Until end of turn, you may play that card. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::HARNESSER_OF_STORMS
};
