use cantrip_api::*;
use cantrip_mechanics::effects::loot;
use cantrip_mechanics::history;
use cantrip_mechanics::statics::have_cant_be_blocked;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

/// "... as long as you've sacrificed an artifact this turn" (CR 611.3a, 701.21a) as the static's `affects`: only this creature, and
/// only while its controller has sacrificed an artifact this turn (`history::sacrificed_this_turn`), judged as it left the battlefield (CR 608.2h): a token, or a
/// permanent that has since stopped being an artifact, still counts. The layers ask again whenever characteristics are computed, so it
/// turns on with the sacrifice and off as the turn ends.
fn itself_while_you_sacrificed_an_artifact(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    if source != affected {
        return false;
    }
    let me = q.state().obj(source).controller;
    history::sacrificed_this_turn(q.state(), q.history(), me, |l| l.types.has(CardType::Artifact))
}

/// "This creature can't be blocked as long as you've sacrificed an artifact this turn." (CR 509.1b, 611.3a, 701.21a): it is
/// unblockable while the condition holds, judged as blockers are declared; a Courier already blocked stays blocked (the
/// ruling). "Whenever this creature attacks, draw a card, then discard a card." is the shared loot (CR 701.9, 121.1).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, loot, "Whenever this creature attacks, draw a card, then discard a card.")],
    statics: &[static_ability(
        Layer::Ability,
        itself_while_you_sacrificed_an_artifact,
        have_cant_be_blocked,
        "This creature can't be blocked as long as you've sacrificed an artifact this turn.",
    )],
    ..oracle::FURTIVE_COURIER
};
