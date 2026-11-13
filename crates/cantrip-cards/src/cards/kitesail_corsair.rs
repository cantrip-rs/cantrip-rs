use cantrip_api::*;
use cantrip_mechanics::statics::have_flying;
use cantrip_oracle as oracle;

/// "This creature ... as long as it's attacking" (CR 506.4, 604.1, 613.1f): the source while it is among the attackers, re-asked on
/// every pass, so it has flying from the declaration of attackers and loses it when removed from combat or combat ends.
fn itself_while_attacking(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && q.state().attackers.iter().any(|a| a.id == source)
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, itself_while_attacking, have_flying, "This creature has flying as long as it's attacking.")],
    ..oracle::KITESAIL_CORSAIR
};
