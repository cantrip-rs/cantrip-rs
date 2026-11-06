use cantrip_api::*;
use cantrip_mechanics::replacements::dealt_to_this_creature_instead;
use cantrip_oracle as oracle;

/// "As long as this creature is untapped, all damage that would be dealt to you by artifacts is dealt to this creature instead." (CR 604.1).
fn as_long_as_creature_untapped_applies(
    q: &dyn Query,
    asker: &Asker,
    proposal: &Proposal,
) -> bool {
    let Proposal::Damage(e) = proposal else { return false };
    let (source, target) = (e.source, &e.target);
    if !q.state().obj(source).zone.is(ZoneKind::Stack)
        && q.chars(source).is(CardType::Artifact)
        && matches!(* target, Target::Player(p) if asker.controller == p)
        && Some(asker.source)
            .is_some_and(|candidate: ObjectId| !(q.state().obj(candidate).tapped))
    {
        return true;
    }
    false
}

pub static CARD: CardDef = CardDef {
    replacements: &[
        replacement(
            ReplacementKind::Damage,
            as_long_as_creature_untapped_applies,
            dealt_to_this_creature_instead,
            "As long as this creature is untapped, all damage that would be dealt to you by artifacts is dealt to this creature instead.",
        ),
    ],
    ..oracle::MARTYRS_OF_KORLIS
};
