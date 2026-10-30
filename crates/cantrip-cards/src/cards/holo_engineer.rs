use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, create a 3/3 colorless Illusion artifact creature token." (CR 603.6a, 111.1).
fn illusion(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ILLUSION_3_3_ARTIFACT, 1)?;
    Ok(())
}

/// "If one or more tokens would be created under your control" (CR 111.1, 614.1a): whose they would be.
fn tokens_under_your_control(_: &dyn Query, me: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::TokenCreation(t) if t.controller == me.controller)
}

/// "... that many 3/3 colorless Illusion artifact creature tokens are created instead": the same count, of a token that
/// is no copy of anything.
fn illusions_instead(q: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::TokenCreation(t) = p {
        t.token = q.card_id(&tokens::ILLUSION_3_3_ARTIFACT);
        t.copy = None;
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        illusion,
        "When this creature enters, create a 3/3 colorless Illusion artifact creature token.",
    )],
    replacements: &[replacement(
        ReplacementKind::TokenCreation,
        tokens_under_your_control,
        illusions_instead,
        "creates 3/3 colorless Illusion artifact creature tokens instead",
    )],
    ..oracle::HOLO_ENGINEER
};

pub static EXTRA: &[&CardDef] = &[&tokens::ILLUSION_3_3_ARTIFACT];
