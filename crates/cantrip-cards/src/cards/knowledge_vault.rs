use cantrip_api::*;
use cantrip_mechanics::{top_of_library, triggers::THIS_LEAVES};
use cantrip_oracle as oracle;

/// Exile the top card without allowing anyone to look at it (CR 406.3, 607.2a).
fn exile_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = top_of_library(g, item.controller, 1);
    g.exile_face_down(&cards, Some(g.linked(item)));
    Ok(())
}

/// Sacrifice on resolution, then discard before returning the linked cards (CR 701.21a, 701.9a, 607.2a).
fn sacrifice_and_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let sacrificed = g.sacrifice(item.controller, &[item.object]);
    g.end_sentence();
    if !sacrificed.is_empty() {
        g.discard_hand(item.controller);
        let cards = g.exiled_with(g.linked(item));
        g.return_to_hand(&cards);
    }
    Ok(())
}

/// The leaves trigger finds cards linked to this same object and text (CR 603.6c, 607.2a).
fn put_exiled_in_graveyards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = g.exiled_with(g.linked(item));
    g.put_into_graveyard(&cards);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}").and(TAP), exile_top, "{2}, {T}: Exile the top card of your library face down."),
        ability(Cost::mana(ManaCost::ZERO), sacrifice_and_return, "{0}: Sacrifice this artifact. If you do, discard your hand, then put all cards exiled with this artifact into their owner's hand."),
        triggered(THIS_LEAVES, put_exiled_in_graveyards, "When this artifact leaves the battlefield, put all cards exiled with it into their owner's graveyard."),
    ],
    ..oracle::KNOWLEDGE_VAULT
};
