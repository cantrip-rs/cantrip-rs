use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Players can't untap more than one artifact during their untap steps." (CR 502.3).
fn players_cant_untap_more_than_untaps_0(_: &dyn Query, _: &Asker, _: PlayerId) -> bool {
    true
}

/// "Players can't untap more than one artifact during their untap steps." (CR 502.3).
fn players_cant_untap_more_than_untapping_0(
    q: &dyn Query,
    _: &Asker,
    id: ObjectId,
) -> bool {
    q.chars(id).is(CardType::Artifact)
}

pub static CARD: CardDef = CardDef {
    untap_limits: &[
        UntapLimit {
            applies: players_cant_untap_more_than_untaps_0,
            permanents: players_cant_untap_more_than_untapping_0,
            amount: 1,
        },
    ],
    ..oracle::DAMPING_FIELD
};
