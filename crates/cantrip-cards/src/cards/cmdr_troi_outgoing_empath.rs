use cantrip_api::*;
use cantrip_mechanics::{choose, targets};
use cantrip_oracle as oracle;

/// "Vigilance
/// {T}: Target opponent whose turn it is may cast a spell from their hand without paying its mana cost. If they do, you copy
/// that spell. You may choose new targets for the copy. If they don't, you draw a card."
///
/// The cast is the targeted opponent's, made while the ability resolves and for nothing (CR 608.2g, 118.9), so it is their
/// card's face and targets that are chosen, and a card they cannot cast is never offered. "If they do" is whether a spell was
/// cast, not whether they were asked: a cast they give up (CR 733.1) leaves the card in hand and draws the controller a card.
/// The copy is not cast (CR 707.10) and is the controller's to retarget (CR 707.10c).
fn troi(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let Some(spell) = choose::may_cast_from_hand_free_spell(g, opponent, item.object, "", |_, _, _| true)? else {
        g.draw(item.controller, 1);
        return Ok(());
    };
    g.copy_spell(spell, item.controller, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, troi, "{T}: Target opponent whose turn it is may cast a spell from their hand without paying its mana cost. If they do, you copy that spell. You may choose new targets for the copy. If they don't, you draw a card.").targets(&[targets::OPPONENT_WHOSE_TURN_IT_IS])],
    ..oracle::CMDR_TROI_OUTGOING_EMPATH
};
