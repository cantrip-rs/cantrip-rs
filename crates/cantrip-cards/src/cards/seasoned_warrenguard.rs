use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "...while you control a token" (CR 111.1): asked as the attack is declared, as part of the trigger event, and never again as the
/// ability resolves - this is no intervening "if" (CR 603.4), so a token that has gone by then changes nothing (the ruling).
fn you_control_a_token(q: &dyn Query, asker: &Asker) -> bool {
    q.permanents_of(asker.controller).into_iter().any(|id| q.obj(id).token)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::this_gets::<2, 0>,
        "Whenever this creature attacks while you control a token, this creature gets +2/+0 until end of turn.",
    )
    .only_if(you_control_a_token)],
    ..oracle::SEASONED_WARRENGUARD
};
