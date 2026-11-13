use cantrip_api::*;
use cantrip_oracle as oracle;

/// "As long as this artifact is untapped, players can't untap more than one land during their untap steps." (CR 502.3).
fn as_long_as_artifact_untapped_untaps_0(
    q: &dyn Query,
    asker: &Asker,
    _: PlayerId,
) -> bool {
    Some(asker.source)
        .is_some_and(|candidate: ObjectId| !(q.state().obj(candidate).tapped))
}

/// "As long as this artifact is untapped, players can't untap more than one land during their untap steps." (CR 502.3).
fn as_long_as_artifact_untapped_untapping_0(
    q: &dyn Query,
    _: &Asker,
    id: ObjectId,
) -> bool {
    q.chars(id).is(CardType::Land)
}

pub static CARD: CardDef = CardDef {
    untap_limits: &[
        UntapLimit {
            applies: as_long_as_artifact_untapped_untaps_0,
            permanents: as_long_as_artifact_untapped_untapping_0,
            amount: 1,
        },
    ],
    ..oracle::WINTER_ORB
};
