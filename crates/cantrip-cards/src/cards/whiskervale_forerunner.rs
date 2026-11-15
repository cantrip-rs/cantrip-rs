use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::triggers::VALIANT;
use cantrip_mechanics::{choose, library, select, top_of_library};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("PutOntoBattlefield", "put the revealed creature card onto the battlefield?", "If you don't, it goes into your hand", "Put it onto the battlefield", "Put it into your hand");

/// "Look at the top five cards of your library. You may reveal a creature card with mana value 3 or less from among them. You may
/// put it onto the battlefield if it's your turn. If you don't put it onto the battlefield, put it into your hand. Put the rest on
/// the bottom of your library in a random order." (CR 701.20a, 401.4): a smaller library shows what it has (CR 609.3). The card is
/// judged by its mana value in the library (CR 202.3), and it is only offered the battlefield on its controller's turn (the check is
/// made as the ability resolves, CR 608.2c). It goes into the hand unless it really arrived on the battlefield, a replacement
/// effect (CR 614) having sent it elsewhere. Every other card, and nothing else, goes to the bottom in a random order.
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let looked = top_of_library(g, you, 5);
    g.look_at(you, &looked);
    let rule = select::creature_cards(select::up_to(1)).with(select::mana_value_at_most(3));
    if let Some(&card) = choose::select(g, you, item.object, Purpose::Reveal, looked.clone(), rule)?.first() {
        g.reveal(you, &[card]);
        let mut on_battlefield = false;
        if g.state().turn.active == you && ask_may(g, you, item.object, Purpose::Ask(&ASK))? {
            let entered = g.put_onto_battlefield(card, you, false)?;
            on_battlefield = g.is_on_battlefield(entered);
        }
        if !on_battlefield {
            g.return_to_hand_revealed(&[card]);
        }
    }
    library::rest_to_bottom_in_random_order(g, you, &looked);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        VALIANT,
        look,
        "Valiant \u{2014} Whenever this creature becomes the target of a spell or ability you control for the first time each turn, look at the top five cards of your library. You may reveal a creature card with mana value 3 or less from among them. You may put it onto the battlefield if it's your turn. If you don't put it onto the battlefield, put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::WHISKERVALE_FORERUNNER
};
