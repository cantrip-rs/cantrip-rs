use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target Villain card in your graveyard" (CR 115.1, 400.7): a card, not a token, in the graveyard of the spell's controller
/// that is a Villain, its creature type read through the words of this spell's text (CR 612).
fn villain_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[])
        && matches!(t, Target::Object(id) if q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Villain)))
}

/// "Target Hero card in your graveyard": see `villain_card_in_your_graveyard`.
fn hero_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[])
        && matches!(t, Target::Object(id) if q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Hero)))
}

/// "Choose one or both" (CR 700.2): each mode has its own target (CR 700.2c), done in printed order (CR 700.2d); one whose target
/// is gone by resolution is left alone, the other still doing its part (CR 608.2b).
static MODAL: ModalDef = modes(&[
    mode(effects::return_first_target_to_hand, "Return target Villain card from your graveyard to your hand.")
        .targets(&[target(villain_card_in_your_graveyard)]),
    mode(effects::return_first_target_to_hand, "Return target Hero card from your graveyard to your hand.")
        .targets(&[target(hero_card_in_your_graveyard)]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::DECOY_PLOY };
