use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS_OR_ATTACKS, YOUR_END_STEP};
use cantrip_mechanics::{choose, history, library};
use cantrip_oracle as oracle;

/// "Each opponent exiles cards from the top of their library until they exile a nonland card" (CR 701.13a, 101.4): in turn
/// order, every card exiled remembered as exiled with this creature (CR 607.2a), the lands too.
fn exile_from_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let by = g.linked(item);
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    for opponent in opponents {
        library::exile_until_any_nonland(g, opponent, Some(by));
    }
    Ok(())
}

/// "You may cast a card exiled with Krang & Shredder without paying its mana cost" (CR 607.2a, 608.2g, 118.9): one of the cards
/// it has exiled that is still in exile and can be cast; a land among them cannot.
fn cast_an_exiled_card_free(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exiled_with(g.linked(item));
    choose::cast_up_to_free(g, item.controller, item.object, &exiled, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS_OR_ATTACKS,
            exile_from_each_opponent,
            "Whenever Krang & Shredder enter or attack, each opponent exiles cards from the top of their library until they exile a nonland card.",
        ),
        triggered(
            YOUR_END_STEP,
            cast_an_exiled_card_free,
            "Disappear \u{2014} At the beginning of your end step, if a permanent left the battlefield under your control this turn, you may cast a card exiled with Krang & Shredder without paying its mana cost.",
        )
        .condition(history::disappear),
    ],
    ..oracle::KRANG_SHREDDER
};
