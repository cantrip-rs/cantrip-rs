use cantrip_api::*;
use cantrip_mechanics::{copy, history, targets};
use cantrip_oracle as oracle;

/// "Target nonland card in a graveyard that was put there from anywhere this turn" (CR 115.1, 400.7): a card, not a land card, in
/// either player's graveyard (`targets::CARD_IN_A_GRAVEYARD`), still the object a move this turn put there (`history::arrived_this_turn`), so a card put there on
/// an earlier turn is not a legal target, and one that has since left is no longer the card that was targeted (CR 608.2b).
fn nonland_card_put_in_a_graveyard_this_turn(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CARD_IN_A_GRAVEYARD.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if !q.chars(id).is(CardType::Land) && history::arrived_this_turn(q, id))
}

/// "Exile target nonland card in a graveyard that was put there from anywhere this turn. Copy it. You may cast the copy without
/// paying its mana cost." The copy is cast while the spell resolves, whatever its timing (CR 608.2g), if it has a legal target and
/// its controller says so; X is 0 and a mandatory additional cost must be paid (the rulings). One not cast ceases to exist
/// (CR 707.10a).
pub static CARD: CardDef = CardDef {
    spell: spell(&[target(nonland_card_put_in_a_graveyard_this_turn)], copy::exile_first_target_and_may_cast_a_copy_without_paying),
    ..oracle::REENACT_THE_CRIME
};
