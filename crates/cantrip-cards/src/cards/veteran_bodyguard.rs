use cantrip_api::*;
use cantrip_mechanics::combat;
use cantrip_oracle as oracle;

/// "As long as this creature is untapped, all damage that would be dealt to you by unblocked creatures is dealt to this creature instead." (CR 604.1).
fn as_long_as_creature_untapped_applies(
    q: &dyn Query,
    asker: &Asker,
    proposal: &Proposal,
) -> bool {
    let Proposal::Damage(e) = proposal else { return false };
    combat::is_unblocked(q, e.source)
        && matches!(e.target, Target::Player(p) if asker.controller == p)
        && !q.state().obj(asker.source).tapped
}

/// "As long as this creature is untapped, all damage that would be dealt to you by unblocked creatures is dealt to this creature instead." (CR 604.1).
fn as_long_as_creature_untapped_apply(
    _: &dyn Query,
    asker: &Asker,
    proposal: &mut Proposal,
) {
    let Proposal::Damage(e) = proposal else { return };
    e.target = Target::Object(asker.source);
}

pub static CARD: CardDef = CardDef {
    replacements: &[
        replacement(
            ReplacementKind::Damage,
            as_long_as_creature_untapped_applies,
            as_long_as_creature_untapped_apply,
            "As long as this creature is untapped, all damage that would be dealt to you by unblocked creatures is dealt to this creature instead.",
        ),
    ],
    ..oracle::VETERAN_BODYGUARD
};
