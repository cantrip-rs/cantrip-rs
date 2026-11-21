use cantrip_api::*;
use cantrip_mechanics::{choose, lands, library, targets, top_of_library};
use cantrip_oracle as oracle;

/// "Strange new worlds — {4}, {T}: Target opponent exiles the top X cards of their library, where X is the number of
/// differently named lands you control. You may cast an instant or sorcery spell from among them without paying its mana
/// cost. Then that player puts the rest on the bottom of their library in a random order." (CR 601.2c, 701.13a, 608.2g, 118.9,
/// 401.4)
///
/// X is counted as the ability resolves, over the ability's controller's lands: the most lands whose names all differ
/// (CR 201.2b), so a second land with a name already counted adds nothing and an opponent's lands are not counted at all. The
/// top X cards of the targeted opponent's library are exiled together (CR 101.4), becoming cards in the exile zone (CR 406.2), and
/// only those that really reached exile (a replacement may send one elsewhere) are among "them". The controller may then cast an instant or sorcery
/// card among them for nothing, as the ability resolves (CR 608.2g, 118.9), choosing one or declining; a card that is not an
/// instant or sorcery is never offered. Then - after that cast, so the card cast is not in exile any more - the opponent puts
/// what is left of them on the bottom of their library in a random order (CR 401.4): the cast card is on the stack or in a
/// graveyard, and one a replacement moved elsewhere is not put back.
fn strange_new_worlds(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let count = lands::differently_named_lands(g, you);
    let top = top_of_library(g, opponent, count);
    let exiled: Vec<ObjectId> = g.exile_all(&top).into_iter().filter(|&card| g.obj(card).zone.is(ZoneKind::Exile)).collect();
    g.end_sentence();
    // "An instant or sorcery spell": the face that would be cast is the spell (CR 709.3a, 715.3a), so an Adventure's instant half counts.
    choose::cast_up_to_among(
        g,
        you,
        item.object,
        Purpose::CastFree,
        &exiled,
        1,
        CastTerms { instead: Some(ManaCost::ZERO), ..CastTerms::default() },
        |g, _, w| {
            let face = g.card(w.face);
            face.is(CardType::Instant) || face.is(CardType::Sorcery)
        },
    )?;
    g.end_sentence();
    library::rest_of_exiled_to_bottom_in_random_order(g, &exiled);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}").and(TAP),
        strange_new_worlds,
        "Strange new worlds \u{2014} {4}, {T}: Target opponent exiles the top X cards of their library, where X is the number of differently named lands you control. You may cast an instant or sorcery spell from among them without paying its mana cost. Then that player puts the rest on the bottom of their library in a random order.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::LIEUTENANT_NYOTA_UHURA
};
