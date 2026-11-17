use cantrip_api::*;
use cantrip_mechanics::copy::create_token_copy_of_target;
use cantrip_mechanics::replacements::artifact_tokens_under_your_control;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens::MAP;
use cantrip_oracle as oracle;

/// "... instead create those tokens plus an additional Map token" (CR 614.1a): one Map more in the same event, however many
/// kinds of artifact token it makes (`TokenCreation::plus`). The Map is created as they are, so what the effect says of them
/// says it of the Map too. It is the Map token Worldwalker Helm's text names now (CR 612.4, 612.2a).
fn plus_a_map(q: &dyn Query, me: &Asker, p: &mut Proposal) {
    if let Proposal::TokenCreation(t) = p {
        let map = q.card_id(&MAP);
        t.plus = Some((map, q.token_copy(map, me.source)));
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{U}, {T}"),
        create_token_copy_of_target,
        "{1}{U}, {T}: Create a token that's a copy of target artifact token you control.",
    )
    .targets(&[targets::ARTIFACT_TOKEN_YOU_CONTROL])],
    replacements: &[replacement(
        ReplacementKind::TokenCreation,
        artifact_tokens_under_your_control,
        plus_a_map,
        "creates those tokens plus an additional Map token instead",
    )],
    ..oracle::WORLDWALKER_HELM
};

pub static EXTRA: &[&CardDef] = &[&MAP];
