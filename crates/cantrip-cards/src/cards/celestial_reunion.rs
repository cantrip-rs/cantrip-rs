use cantrip_api::*;
use cantrip_mechanics::{behold, library, select};
use cantrip_oracle as oracle;

/// "Search your library for a creature card with mana value X or less, reveal it, put it into your hand, then shuffle. If this
/// spell's additional cost was paid and the revealed card is the chosen type, put that card onto the battlefield instead of
/// putting it into your hand" (CR 701.23, 701.20): the type was chosen as the cost was announced, and only a paid cost chose
/// one (`StackItem::announced_type`). The card is judged as it was in the library, where an {X} in its cost is 0 (CR 202.3e).
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let rule = select::creature_cards(select::up_to(1)).with(select::mana_value_at_most(item.x as u32));
    let found = library::search(g, you, you, item.object, SearchDestination::Found, rule)?;
    g.reveal(you, &found);
    if let Some(&card) = found.first() {
        if item.announced_type().is_some_and(|chosen| g.chars(card).has_subtype(chosen)) {
            g.put_onto_battlefield(card, you, false)?;
        } else {
            g.return_to_hand_revealed(&[card]);
        }
    }
    g.shuffle(you);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    optional_cost: Some(behold::optional_of_chosen_type(2)),
    spell: spell(&[], search),
    labels: &["Cast without choosing a type", "Choose a creature type and behold two creatures of that type"],
    ..oracle::CELESTIAL_REUNION
};
