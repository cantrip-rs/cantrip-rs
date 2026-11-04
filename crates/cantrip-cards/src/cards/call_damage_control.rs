use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets::{self, card_in_your_graveyard};
use cantrip_oracle as oracle;

fn artifact_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_in_your_graveyard(q, asker, t, &[CardType::Artifact])
}

fn enchantment_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_in_your_graveyard(q, asker, t, &[CardType::Enchantment])
}

/// "Choose up to two." (CR 700.2): the modes only name their targets, a card of that type in your graveyard each (CR 700.2c); the
/// instruction before them, "Return those cards from your graveyard to your hand.", is done once to all of them
/// (`ModalDef::shared`): the targets of the chosen modes that are still legal, returned together as one event (CR 400.3, 608.2b).
static MODAL: ModalDef = modes(&[
    mode(no_effect, "Target artifact card.").targets(&[target(artifact_card_in_your_graveyard)]),
    mode(no_effect, "Target creature card.").targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    mode(no_effect, "Target enchantment card.").targets(&[target(enchantment_card_in_your_graveyard)]),
    mode(no_effect, "Target land card.").targets(&[targets::LAND_CARD_IN_YOUR_GRAVEYARD]),
])
.choose(0, 2)
.shared(effects::return_targets_to_hand);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::CALL_DAMAGE_CONTROL };
