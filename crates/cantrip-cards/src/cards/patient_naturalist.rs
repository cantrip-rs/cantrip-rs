use cantrip_api::*;
use cantrip_mechanics::select::{self, exactly};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{choose, tokens};
use cantrip_oracle as oracle;

/// "When this creature enters, mill three cards. Put a land card from among the milled cards into your hand. If you can't,
/// create a Treasure token." (CR 701.17, 608.2c, 111.10a): the land card is not optional and is taken from the cards the mill
/// moved, as they are where they went (a graveyard, or exile by a replacement effect), never from a land card that was in the
/// graveyard already. When none of them is a land card the instruction can't be carried out and the Treasure is made instead.
fn enter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let milled = g.mill(you, 3);
    g.end_sentence();
    if milled.iter().any(|&card| g.chars(card).is(CardType::Land)) {
        let chosen = choose::select(g, you, item.object, Purpose::PutIntoHand, milled, select::land_cards(exactly(1)))?;
        g.return_to_hand(&chosen);
    } else {
        tokens::create_treasure(g, you)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        enter,
        "When this creature enters, mill three cards. Put a land card from among the milled cards into your hand. If you can't, create a Treasure token.",
    )],
    ..oracle::PATIENT_NATURALIST
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
