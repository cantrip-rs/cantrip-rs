use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets, triggers};
use cantrip_oracle as oracle;

pub static EMBERETH_BLAZE: CardDef = CardDef { spell: spell(&[targets::ANY], effects::damage_first_target::<2>), ..oracle::EMBERETH_BLAZE };

const ASK: Question = Question::new("ExileTop", "exile that many cards from the top of your library?", "You may play them this turn", "Exile them", "Don't");

/// "You may exile that many cards from the top of your library. You may play those cards this turn." (CR 305.1, 601.3, 611.2): that
/// many is the damage dealt, asked as the trigger resolves; the cards are played, not only cast, so a land among them may be the
/// turn's land drop. A library with fewer cards exiles what it has.
fn exile_that_many(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        library::exile_top_play_this_turn(g, item.controller, item.matched_damage_amount() as usize);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    adventure: Some(&EMBERETH_BLAZE),
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, triggers::source_you_control_deals_noncombat_damage_to_an_opponent),
        exile_that_many,
        "Whenever a source you control deals noncombat damage to an opponent, you may exile that many cards from the top of your library. You may play those cards this turn.",
    )
    .damage_amount()],
    ..oracle::VIRTUE_OF_COURAGE
};

pub static EXTRA: &[&CardDef] = &[&EMBERETH_BLAZE];
