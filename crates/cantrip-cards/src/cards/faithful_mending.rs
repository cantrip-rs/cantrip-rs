use cantrip_api::*;
use cantrip_mechanics::{effects, flashback};
use cantrip_oracle as oracle;

/// "You gain 2 life, draw two cards, then discard two cards." (CR 119.3, 121.1, 701.9): the life is gained first, then the
/// two cards are drawn, and only then are the two chosen from the hand that now holds them.
fn mending(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 2)?;
    g.end_sentence();
    effects::draw_then_discard::<2, 2>(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], mending),
    ways: &[flashback::way(
        cost("{1}{W}{U}"),
        "Flashback {1}{W}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
    )],
    ..oracle::FAITHFUL_MENDING
};
