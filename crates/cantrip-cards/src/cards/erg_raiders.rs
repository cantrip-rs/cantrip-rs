use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// The intervening condition is checked both when the ability triggers and when it resolves (CR 603.4).
fn didnt_attack(q: &dyn Query, asker: &Asker) -> bool {
    !q.history().attacked_this_turn(asker.source)
}

/// "It deals 2 damage to you unless it came under your control this turn": it entered the battlefield this turn (the
/// ability's controller controlled it when it triggered, CR 603.3a), or an effect gave it to that player this turn. Read
/// from the history when the ability resolves, so a later control change does not erase an arrival (CR 608.2h).
fn deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let history = g.history();
    let came_under_control = history.entered_this_turn(item.object)
        || history.select(EventQuery::kind(EventMask::CONTROL_CHANGED).object(item.object).this_turn())
            .any(|r| matches!(r.event, Event::ControlChanged { controller, .. } if controller == item.controller));
    if !came_under_control {
        g.damage_together(&[(item.object, Target::Player(item.controller), 2)]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_END_STEP, deal_damage,
        "At the beginning of your end step, if this creature didn't attack this turn, it deals 2 damage to you unless it came under your control this turn.")
        .condition(didnt_attack)],
    ..oracle::ERG_RAIDERS
};
