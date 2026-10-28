use cantrip_api::*;
use cantrip_mechanics::delayed::{self, exile_target_and_return_at_next_end_step};
use cantrip_mechanics::targets::nonland_nontoken_permanent_filter;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST;
use cantrip_oracle as oracle;

/// "Another target nonland, nontoken permanent": any such permanent on the battlefield but Wiccan itself.
fn another_nonland_nontoken_permanent(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if id != asker.source) && nonland_nontoken_permanent_filter(q, asker, t)
}

pub static CARD: CardDef = CardDef {
    // "Exile another target nonland, nontoken permanent. Return that card to the battlefield under its owner's control at the
    // beginning of the next end step." (CR 603.7): a delayed ability about the card exiled.
    abilities: &[triggered(
        NONCREATURE_SPELL_YOU_CAST,
        exile_target_and_return_at_next_end_step,
        "Whenever you cast a noncreature spell, exile another target nonland, nontoken permanent. Return that card to the battlefield under its owner's control at the beginning of the next end step.",
    )
    .targets(&[target(another_nonland_nontoken_permanent)])],
    delayed: &[delayed_trigger(
        delayed::return_subject_to_battlefield,
        "Return the card exiled with Wiccan, Rising Magician to the battlefield under its owner's control.",
    )],
    ..oracle::WICCAN_RISING_MAGICIAN
};
