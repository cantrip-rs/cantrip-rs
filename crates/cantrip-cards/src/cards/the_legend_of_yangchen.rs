use cantrip_api::*;
use cantrip_mechanics::{airbend_targets, choose, effects, exile_and_return_transformed, flurry, select, targets};
use cantrip_oracle as oracle;

/// "Starting with you, each player chooses up to one permanent with mana value 3 or greater from among permanents your
/// opponents control. Exile those permanents." (CR 608.2d, 101.4, 406.1.) The players choose in turn order from the Saga's
/// controller; the same permanent may be chosen by more than one of them. Nothing is exiled until everyone has chosen, and
/// the permanents chosen leave together.
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let candidates: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.controller(id) != you && g.mana_value(id) >= 3)
        .collect();
    let choosers: Vec<PlayerId> = g.state().turn_order_from(you).collect();
    let mut chosen: Vec<ObjectId> = Vec::new();
    for player in choosers {
        let picked = choose::select(g, player, item.object, Purpose::Exile, candidates.clone(), select::cards(select::up_to(1)))?;
        for id in picked {
            if !chosen.contains(&id) {
                chosen.push(id);
            }
        }
    }
    g.exile_all(&chosen);
    Ok(())
}

const ASK: Question = Question::new(
    "OpponentDrawsThree",
    "have the targeted opponent draw three cards?",
    "If you do, you draw three cards",
    "They draw three",
    "Don't",
);

/// "You may have target opponent draw three cards. If you do, draw three cards." (CR 608.2b, 608.2d.) The opponent is chosen as
/// the chapter goes on the stack; if they are no longer a legal target nothing happens and the choice is not offered. The
/// "may" is decided as it resolves, and only the opponent's draw is optional, so yours depends on it (CR 608.2c).
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.draw(opponent, 3);
        g.draw(item.controller, 3);
    }
    Ok(())
}

fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_and_return_transformed(g, item.object, item.controller)
}

pub static BACK: CardDef = CardDef {
    abilities: &[flurry::trigger(
        "Whenever you cast your second spell each turn, airbend up to one other target nonland permanent. (Exile it. While it's exiled, its owner may cast it for {2} rather than its mana cost.)",
        airbend_targets,
    )
    .targets(&[up_to_one(targets::other_nonland_permanent_filter)])],
    ..oracle::AVATAR_YANGCHEN
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            chapter_one,
            "I \u{2014} Starting with you, each player chooses up to one permanent with mana value 3 or greater from among permanents your opponents control. Exile those permanents.",
        ),
        chapter(2, chapter_two, "II \u{2014} You may have target opponent draw three cards. If you do, draw three cards.").targets(&[targets::OPPONENT]),
        chapter(3, chapter_three, "III \u{2014} Exile this Saga, then return it to the battlefield transformed under your control."),
    ],
    back: Some(&BACK),
    ..oracle::THE_LEGEND_OF_YANGCHEN
};

pub static EXTRA: &[&CardDef] = &[];
