use cantrip_api::*;
use cantrip_mechanics::library::reveal_top_select_to_hand_rest_to_bottom_in_random_order;
use cantrip_mechanics::select::{self, part};
use cantrip_mechanics::tokens::{self, SOLDIER};
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Creature and/or land cards" (CR 205.2a): a card that is a creature card, a land card, or both, as it is in the library.
fn creature_or_land(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| {
        let chars = q.chars(card);
        chars.is(CardType::Creature) || chars.is(CardType::Land)
    })
}

/// "Reveal the top seven cards of your library. Put any number of creature and/or land cards from among them into
/// your hand and the rest on the bottom of your library in a random order."
fn reveal_seven(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::cards(select::ANY_NUMBER).with(part(&(creature_or_land as SelectFn), 0, "")).called("creature and/or land");
    reveal_top_select_to_hand_rest_to_bottom_in_random_order(g, item.controller, item.object, 7, rule)?;
    Ok(())
}

fn create_soldiers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SOLDIER, 7)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&SOLDIER];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, reveal_seven, "When Torsten enters, reveal the top seven cards of your library. Put any number of creature and/or land cards from among them into your hand and the rest on the bottom of your library in a random order."),
        triggered(triggers::THIS_DIES, create_soldiers, "When Torsten dies, create seven 1/1 white Soldier creature tokens."),
    ],
    ..oracle::TORSTEN_FOUNDER_OF_BENALIA
};
