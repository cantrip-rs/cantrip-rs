use cantrip_api::*;
use cantrip_mechanics::{each_opponent_sacrifices, effects, exile_and_return_transformed, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ExileAndReturn", "exile Jecht, then return him to the battlefield transformed?", "He returns as Braska's Final Aeon", "Exile and transform", "Don't");

/// "You may exile it, then return it to the battlefield transformed under its owner's control." The choice is asked
/// whether or not he is still there; if he is not, there is nothing to exile (CR 400.7).
fn exile_and_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

// --- Braska's Final Aeon -----------------------------------------------------------------------------------------

/// I, II: "Each opponent discards a card and you draw a card." (CR 701.9, 121.1), in that order.
fn jecht_beam(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::each_opponent_discards_a_card(g, item)?;
    effects::draw(g, item, 1)
}

/// III: "Each opponent sacrifices two creatures of their choice." (CR 701.21a); one with two or fewer sacrifices them all.
fn ultimate_jecht_shot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_opponent_sacrifices(g, item.controller, item.object, 2, |q, id| q.chars(id).is(CardType::Creature))
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        chapter(1, jecht_beam, "I \u{2014} Jecht Beam \u{2014} Each opponent discards a card and you draw a card."),
        chapter(2, jecht_beam, "II \u{2014} Jecht Beam \u{2014} Each opponent discards a card and you draw a card."),
        chapter(3, ultimate_jecht_shot, "III \u{2014} Ultimate Jecht Shot \u{2014} Each opponent sacrifices two creatures of their choice."),
    ],
    ..oracle::BRASKAS_FINAL_AEON
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT,
        exile_and_return,
        "Whenever Jecht deals combat damage to a player, you may exile it, then return it to the battlefield transformed under its owner's control.",
    )],
    back: Some(&BACK),
    ..oracle::JECHT_RELUCTANT_GUARDIAN
};
