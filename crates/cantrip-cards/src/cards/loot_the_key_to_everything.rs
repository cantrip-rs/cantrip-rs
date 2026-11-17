use cantrip_api::*;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

/// "Exile the top X cards of your library, where X is the number of card types among other nonland permanents you control. You may
/// play those cards this turn." (CR 205.2a, 608.2h, 701.13, 305.1, 601.3): X is worked out as the ability resolves. A permanent
/// that is a land is disregarded altogether, an artifact land's artifact type too, as is Loot itself; each card type is counted once
/// however many permanents have it, and tokens count. The cards are exiled even from a library of fewer; a land among them may be
/// played, under the ordinary land-play rules, until the end of the turn.
fn exile_top_cards_for_each_card_type(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let others: Vec<Chars> = g.permanents_of(you).into_iter().filter(|&id| id != item.object).map(|id| g.chars(id)).filter(|c| !c.is(CardType::Land)).collect();
    let x = CardType::ALL.iter().filter(|&&kind| others.iter().any(|c| c.is(kind))).count();
    library::exile_top_play_this_turn(g, you, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)"),
        triggered(
            triggers::YOUR_UPKEEP,
            exile_top_cards_for_each_card_type,
            "At the beginning of your upkeep, exile the top X cards of your library, where X is the number of card types among other nonland permanents you control. You may play those cards this turn.",
        ),
    ],
    ..oracle::LOOT_THE_KEY_TO_EVERYTHING
};
