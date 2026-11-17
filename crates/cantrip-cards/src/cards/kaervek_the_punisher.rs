use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_COMMIT_A_CRIME;
use cantrip_mechanics::{copy, targets};
use cantrip_oracle as oracle;

/// "Target black card from your graveyard" (CR 115.1, 105.2): a card there, not a copy or a token, that is black (the
/// colors it has in the graveyard, CR 109.3); a colorless or multicolor card with black in it counts only when black.
fn black_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[])
        && matches!(t, Target::Object(id) if q.chars(id).colors.has(q.words(asker.source).color(Color::Black)))
}

/// "Exile up to one target black card from your graveyard and copy it. You may cast the copy. If you do, you lose 2 life." (CR 701.13a,
/// 707.12, 608.2g): the target is exiled, a new object (CR 400.7), and "it" is the card in exile, copied there; the copy is cast as the
/// ability resolves, paying its costs, or not at all (a copy of a permanent spell becomes a token, CR 707.10f). The life is lost only
/// when the copy was cast (CR 608.2c). With no target, or one gone as the ability resolves, nothing happens (CR 608.2b).
fn exile_copy_cast_lose_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile(card);
    if copy::may_cast_a_copy_paying_costs(g, item.controller, exiled)? {
        g.end_sentence();
        g.lose_life(item.controller, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_COMMIT_A_CRIME,
        exile_copy_cast_lose_life,
        "Whenever you commit a crime, exile up to one target black card from your graveyard and copy it. You may cast the copy. If you do, you lose 2 life.",
    )
    .targets(&[target(black_card_in_your_graveyard).optional()])],
    ..oracle::KAERVEK_THE_PUNISHER
};
