use cantrip_api::*;
use cantrip_mechanics::targets::{self, card_in_your_graveyard};
use cantrip_oracle as oracle;

fn artifact_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_in_your_graveyard(q, asker, t, &[CardType::Artifact])
}

fn enchantment_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_in_your_graveyard(q, asker, t, &[CardType::Enchantment])
}

/// "Return those cards from your graveyard to your hand." (CR 400.3, 700.2, 608.2b): done once, to the targets of the chosen
/// modes that are still legal, all returned at the same time as one event; a card that is gone from the graveyard is left out
/// and the others still return.
fn return_those_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = targets::legal_objects(g, item);
    g.return_to_hand(&cards);
    Ok(())
}

/// "Choose up to two." (CR 700.2): the modes only name their targets, a card of that type in your graveyard each (CR 700.2c); the
/// instruction before them is done once to all of them (`ModalDef::shared`).
static MODAL: ModalDef = modes(&[
    mode(no_effect, "Target artifact card.").targets(&[target(artifact_card_in_your_graveyard)]),
    mode(no_effect, "Target creature card.").targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    mode(no_effect, "Target enchantment card.").targets(&[target(enchantment_card_in_your_graveyard)]),
    mode(no_effect, "Target land card.").targets(&[targets::LAND_CARD_IN_YOUR_GRAVEYARD]),
])
.choose(0, 2)
.shared(return_those_cards);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::CALL_DAMAGE_CONTROL };
