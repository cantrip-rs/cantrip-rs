use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::replacements::instead_of_drawing;
use cantrip_oracle as oracle;

/// "The next time you would draw a card this turn": a draw of the player who activated the ability (CR 121.6), the first
/// one after it resolved that can happen (CR 614.17c).
fn you_would_draw(_: &dyn Query, asker: &Asker, _: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Draw(d) if d.player == asker.controller)
}

fn instead(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    instead_of_drawing(q, asker, proposal);
}

/// "... each player returns a permanent they control to its owner's hand instead" (CR 101.4): in turn order from the active
/// player, each chooses one of their permanents (nothing is asked of a player with none), and they are returned together.
fn each_returns(g: &mut dyn Game, asker: &Asker, _: ObjectId, _: &Proposal, _: &Proposal) {
    let _ = each_player_returns_a_permanent(g, asker.source);
}

fn each_player_returns_a_permanent(g: &mut dyn Game, source: ObjectId) -> Result<(), Suspend> {
    let mut chosen = Vec::new();
    for player in g.state().apnap().collect::<Vec<_>>() {
        if !cantrip_mechanics::players::reached(g, player) {
            continue;
        }
        let theirs = g.permanents_of(player);
        chosen.extend(choose::card(g, player, source, Purpose::ReturnToHand, theirs)?);
    }
    g.return_to_hand(&chosen);
    Ok(())
}

/// "{1}: The next time you would draw a card this turn, ... instead": a replacement effect about the player, lasting the
/// turn and used up by the one draw it replaces (CR 614.1a, 611.2). Each activation makes one; with several, the player
/// chooses which applies to a draw (its ruling, CR 616.1).
fn shield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_player_effect(item.controller, item.object, EffectKind::ReplacementRule { card: item.ability.card, index: 0, value: 0 }, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}"),
        shield,
        "{1}: The next time you would draw a card this turn, each player returns a permanent they control to its owner's hand instead.",
    )],
    granted_replacements: &[granted_replacement(
        ReplacementKind::Draw,
        you_would_draw,
        instead,
        "instead, each player returns a permanent they control to its owner's hand",
    )
    .then(each_returns)
    .once()
    .text("The next time you would draw a card this turn, each player returns a permanent they control to its owner's hand instead.")],
    ..oracle::WORDS_OF_WIND
};
