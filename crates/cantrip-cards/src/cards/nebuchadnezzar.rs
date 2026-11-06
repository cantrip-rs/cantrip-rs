use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// Name a card, reveal up to X random cards in the opponent's hand, then discard the matching revealed cards (CR 701.9c, 701.20a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let Answer::Name(Some(name)) = g.ask(item.controller, RequestKind::ChooseName {
        source: item.object,
        looked_at: Vec::new(),
        suggestion: None,
        kind: NameKind::ANY,
    })? else { return Ok(()) };
    g.end_sentence();
    let mut pool = g.state().zone(Zone::hand(opponent)).to_vec();
    let mut shown = Vec::new();
    while shown.len() < item.x as usize && !pool.is_empty() {
        let Some(card) = library::random_of(g, &pool) else { break };
        shown.push(card);
        pool.retain(|&id| id != card);
    }
    g.reveal(opponent, &shown);
    g.end_sentence();
    let matching: Vec<_> = shown.into_iter().filter(|&id| library::object_has_name(g, id, name)).collect();
    g.discard_revealed(&matching);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{X}").and(TAP),
        effect,
        "{X}, {T}: Choose a card name. Target opponent reveals X cards at random from their hand. Then that player discards all cards with that name revealed this way. Activate only during your turn.",
    )
    .targets(&[targets::OPPONENT])
    .timing(Timing::OwnTurn)],
    ..oracle::NEBUCHADNEZZAR
};
