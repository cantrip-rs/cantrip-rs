use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::history::creatures_attacked_with;
use cantrip_mechanics::replacements::tokens_under_your_control;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "If one or more creature tokens would be created under your control" (CR 111.1, 614.1a): tokens that are yours and, as they
/// would be created, creatures (a copy exception that makes one a creature counts, CR 707.9b). `tokens_under_your_control`,
/// narrowed to creatures.
fn creature_tokens_under_your_control(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    tokens_under_your_control(q, asker, proposal) && matches!(proposal, Proposal::TokenCreation(t) if q.token_chars(t.token, t.copy).is(CardType::Creature))
}

/// "... three times that many of those tokens are created instead." Whatever the creation says of the tokens (tapped and
/// attacking, counters) holds for all of them (the ruling).
fn three_times_that_many_tokens(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::TokenCreation(t) = proposal {
        t.count = t.count * 3;
    }
}

/// "Activate only if you attacked with three or more creatures this turn" (CR 508.1, 602.5b): the creatures the ability's controller
/// declared as attackers this turn, each counted once.
fn you_attacked_with_three_or_more_creatures(q: &dyn Query, me: &Asker) -> bool {
    creatures_attacked_with(q, me.controller) >= 3
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::W), "{T}: Add {W}."),
        ability(cost("{2}{W}, {T}"), effects::transform_self, "{2}{W}, {T}: Transform this land. Activate only if you attacked with three or more creatures this turn and only as a sorcery.")
            .sorcery_speed()
            .condition(you_attacked_with_three_or_more_creatures),
    ],
    ..oracle::TEMPLE_OF_CIVILIZATION
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, effects::return_it_tapped_and_transformed, "When Ojer Taq dies, return it to the battlefield tapped and transformed under its owner's control.")],
    replacements: &[replacement(
        ReplacementKind::TokenCreation,
        creature_tokens_under_your_control,
        three_times_that_many_tokens,
        "three times that many creature tokens are created",
    )],
    back: Some(&BACK),
    ..oracle::OJER_TAQ_DEEPEST_FOUNDATION
};
