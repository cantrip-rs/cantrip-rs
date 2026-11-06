use cantrip_api::*;
use cantrip_mechanics::{library, targets, top_of_library};
use cantrip_oracle as oracle;

/// The targeted player names a card, then reveals and moves their top card (CR 201.4, 701.20a).
fn name_and_reveal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let Answer::Name(Some(name)) = g.ask(player, RequestKind::ChooseName {
        source: item.object, looked_at: Vec::new(), suggestion: None, kind: NameKind::ANY,
    })? else { return Ok(()) };
    let Some(&card) = top_of_library(g, player, 1).first() else { return Ok(()) };
    g.reveal(player, &[card]);
    g.end_sentence();
    if library::object_has_name(g, card, name) {
        g.return_to_hand_revealed(&[card]);
    } else {
        g.put_into_graveyard(&[card]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, name_and_reveal,
        "{T}: Target player chooses a card name, then reveals the top card of their library. If that card has the chosen name, that player puts it into their hand. If it doesn't, the player puts it into their graveyard.")
        .targets(&[targets::PLAYER])],
    ..oracle::PETRA_SPHINX
};
