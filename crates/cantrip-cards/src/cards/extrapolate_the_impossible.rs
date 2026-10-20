use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "You may reveal exactly two cards you own with different names from outside the game. An opponent chooses one of them.
/// You put that card into your hand."
///
/// Reveal the owned pair together; only the opponent's pick enters the game (CR 108.3b, 400.11, 701.20).
fn reveal_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let outside = g.outside_the_game(you);
    let Some([first, second]) = pair(g, you, item.object, &outside)? else {
        return Ok(());
    };
    g.reveal(you, &[first, second]);
    if let Some(card) = choose::opponent_chooses_one(g, you, item.object, vec![first, second])? {
        g.bring_into_hand(&[card]);
    }
    Ok(())
}

/// Choose a pairable first card, then a differently named partner (CR 608.2d).
/// Declining or having no legal pair returns `None`; neither card is revealed yet.
fn pair(g: &mut dyn Game, player: PlayerId, source: ObjectId, outside: &[ObjectId]) -> Result<Option<[ObjectId; 2]>, Suspend> {
    let pairable: Vec<ObjectId> = outside.iter().copied().filter(|&card| outside.iter().any(|&other| name_of(g, other) != name_of(g, card))).collect();
    let Some(first) = choose::cards(g, player, source, Purpose::Reveal, pairable, 1, true)?.first().copied() else {
        return Ok(None);
    };
    let partners: Vec<ObjectId> = outside.iter().copied().filter(|&card| name_of(g, card) != name_of(g, first)).collect();
    Ok(choose::cards(g, player, source, Purpose::Reveal, partners, 1, false)?.first().copied().map(|second| [first, second]))
}

/// A card's printed name outside the game (CR 201.2).
fn name_of(g: &dyn Game, card: ObjectId) -> &'static str {
    g.card(g.obj(card).card).name
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], reveal_two),
    ..oracle::EXTRAPOLATE_THE_IMPOSSIBLE
};
