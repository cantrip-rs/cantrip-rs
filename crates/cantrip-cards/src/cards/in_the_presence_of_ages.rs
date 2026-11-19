use cantrip_api::*;
use cantrip_mechanics::{choose, top_of_library};
use cantrip_oracle as oracle;

/// "Reveal the top four cards of your library. You may put a creature card and/or a land card from among them into your hand. Put
/// the rest into your graveyard." (CR 701.20a, 608.2d, 404.3): up to one of each, both chosen with the four cards in view before
/// either moves (the ruling: none, a creature, a land, or both). A card that is both (Dryad Arbor) can be the creature or the land
/// but not both, so the land is chosen from the rest. The cards taken go to the hand revealed; the others go to the graveyard together.
fn reveal_four_and_take_a_creature_and_or_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let revealed = top_of_library(g, you, 4);
    g.reveal(you, &revealed);
    let taken = choose::a_creature_card_and_or_a_land_card(g, you, item.object, Purpose::PutIntoHand, revealed.clone())?;
    g.return_to_hand_revealed(&taken);
    let rest: Vec<ObjectId> = revealed.into_iter().filter(|card| !taken.contains(card)).collect();
    g.put_into_graveyard(&rest);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], reveal_four_and_take_a_creature_and_or_a_land), ..oracle::IN_THE_PRESENCE_OF_AGES };
