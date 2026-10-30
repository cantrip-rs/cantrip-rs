use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// The 1/1 black and green Worm creature token.
pub static WORM: CardDef = CardDef {
    // Black (bit 2) and green (bit 4).
    colors: Some(ColorSet(0b10100)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Worm)
};

/// "Mill four cards, then you may put a creature or land card from among the milled cards on top of your library" (CR 701.17,
/// 401.3): the cards are found where they went (a graveyard, or exile by a replacement effect), each judged by the types it has
/// there; nobody is asked when none qualifies, and choosing none declines.
fn mill_and_put_back(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let milled = g.mill(player, 4);
    let takeable: Vec<ObjectId> =
        milled.iter().copied().filter(|&c| g.chars(c).is(CardType::Creature) || g.chars(c).is(CardType::Land)).collect();
    let chosen = cantrip_mechanics::choose::cards_from_kind(g, player, item.object, Purpose::PutOnTop, "creature or land", milled, takeable, 1, true)?;
    g.put_into_library(&chosen, LibraryPlace::Top);
    Ok(())
}

/// "Discard a land card": a land card in the activator's hand.
fn land_card(q: &dyn Query, _: &Asker, card: ObjectId) -> bool {
    q.chars(card).is(CardType::Land)
}

/// "Create a 1/1 black and green Worm creature token for each land card in your graveyard" (CR 701.7a): counted as the ability
/// resolves, the land card discarded as its cost included (CR 608.2c); a token is never a card in a graveyard.
fn worms_for_lands_in_your_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let lands = g.state().zone(Zone::graveyard(player)).iter().filter(|&&c| g.chars(c).is(CardType::Land)).count() as u32;
    tokens::create(g, player, &WORM, lands)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            mill_and_put_back,
            "When this creature enters, mill four cards, then you may put a creature or land card from among the milled cards on top of your library.",
        ),
        ability(
            cost("{2}{B/G}{B/G}{B/G}, {T}").and(discard(1, land_card)),
            worms_for_lands_in_your_graveyard,
            "{2}{B/G}{B/G}{B/G}, {T}, Discard a land card: Create a 1/1 black and green Worm creature token for each land card in your graveyard.",
        ),
    ],
    ..oracle::LLUWEN_IMPERFECT_NATURALIST
};

pub static EXTRA: &[&CardDef] = &[&WORM];
