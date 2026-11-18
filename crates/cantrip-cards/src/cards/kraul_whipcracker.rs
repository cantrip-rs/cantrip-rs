use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "target token an opponent controls" (CR 115.1, 111.1, 110.2): a token on the battlefield under an opponent's control, judged again as
/// the ability resolves (CR 608.2b).
fn token_an_opponent_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::PERMANENT_AN_OPPONENT_CONTROLS.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).token)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::destroy_first_target, "When this creature enters, destroy target token an opponent controls.")
        .targets(&[target(token_an_opponent_controls)])],
    ..oracle::KRAUL_WHIPCRACKER
};
