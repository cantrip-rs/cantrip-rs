use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Activated abilities of artifacts can't be activated." Only an artifact permanent on the battlefield is affected;
/// abilities in other zones are not (CR 602.5, 109.2).
fn artifacts_cant_activate(q: &dyn Query, _: &Asker, _: PlayerId, source: ObjectId, _: &ActivatedDef) -> bool {
    !q.is_on_battlefield(source) || !q.chars(source).is(CardType::Artifact)
}

pub static CARD: CardDef = CardDef {
    activation_rules: &[artifacts_cant_activate],
    ..oracle::COLLECTOR_OUPHE
};
