use cantrip_api::*;
use cantrip_mechanics::{effects::return_first_target_to_hand_then_plus_one_counter_on_this, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Up to one target non-Faerie, nonland permanent you control": the Faerie word is read through the words of Nurturing
/// Pixie's text (CR 612).
fn non_faerie_nonland_permanent_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::nonland_permanent_you_control_filter(q, asker, t)
        && matches!(t, Target::Object(id) if !q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Faerie)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, return_first_target_to_hand_then_plus_one_counter_on_this, "When this creature enters, return up to one target non-Faerie, nonland permanent you control to its owner's hand. If a permanent was returned this way, put a +1/+1 counter on this creature.")
        .targets(&[up_to_one(non_faerie_nonland_permanent_you_control)])],
    ..oracle::NURTURING_PIXIE
};
