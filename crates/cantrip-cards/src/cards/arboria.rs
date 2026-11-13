use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Creatures can't attack a player unless that player cast a spell or put a nontoken permanent onto the battlefield during their last turn." (CR 508.1c).
fn creatures_cant_attack_player_unless_rule(
    q: &dyn Query,
    _: &Asker,
    _: ObjectId,
    defender: Target,
) -> bool {
    let Target::Player(player) = defender else { return true };
    let history = q.history();
    let last_turn = history.all().iter().rev()
        .find(|r| r.active == player && r.turn < q.state().turn.number)
        .map(|r| r.turn);
    history.all().iter().filter(|r| Some(r.turn) == last_turn).any(|r| match r.event {
        Event::SpellCast { player: caster, .. } => caster == player,
        Event::ZoneChange { new, to, put_by: Some(putter), .. } =>
            putter == player && to.is(ZoneKind::Battlefield) && !q.state().obj(new).token,
        _ => false,
    })
}

pub static CARD: CardDef = CardDef {
    attack_rules: &[creatures_cant_attack_player_unless_rule],
    ..oracle::ARBORIA
};
