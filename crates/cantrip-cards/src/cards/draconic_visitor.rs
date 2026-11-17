use cantrip_api::*;
use cantrip_mechanics::replacements::artifact_tokens_under_your_control;
use cantrip_mechanics::tokens::DRAGON_5_5;
use cantrip_oracle as oracle;

/// "... that many 5/5 red Dragon creature tokens with flying are created instead": the same count, of a token that is
/// no copy of anything, but what Draconic Visitor's text says now (CR 612.4, 612.2a: Sleight of Mind's blue, Artificial
/// Evolution's Goblin).
fn dragons_instead(q: &dyn Query, me: &Asker, p: &mut Proposal) {
    if let Proposal::TokenCreation(t) = p {
        t.token = q.card_id(&DRAGON_5_5);
        t.copy = q.token_copy(t.token, me.source);
    }
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::TokenCreation,
        artifact_tokens_under_your_control,
        dragons_instead,
        "creates 5/5 red Dragon creature tokens with flying instead",
    )],
    ..oracle::DRACONIC_VISITOR
};

pub static EXTRA: &[&CardDef] = &[&DRAGON_5_5];
