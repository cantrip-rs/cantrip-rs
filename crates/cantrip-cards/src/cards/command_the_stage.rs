use cantrip_api::*;
use cantrip_api::ZoneKind::Graveyard;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 2/2 colorless Wizard Soldier creature token named Cadet, then put a +1/+1 counter on each other
/// Wizard token you control." Create the Cadet before counting the other Wizard tokens. "Wizard" is read through this
/// card's words (CR 612.2).
fn command_the_stage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cadets = tokens::create(g, item.controller, &tokens::CADET, 1)?;
    let wizard = g.words(item.object).subtype(Subtype::Wizard);
    for token in g.permanents_of(item.controller) {
        if !cadets.contains(&token) && g.obj(token).token && g.chars(token).has_subtype(wizard) {
            g.add_counters(token, CounterKind::PlusOne, 1);
        }
    }
    Ok(())
}

/// "At the beginning of each upkeep" (CR 503.1a): any player's upkeep, with this card in the graveyard, since it is the only
/// zone its ability works from (`.anywhere()`).
fn each_upkeep_in_the_graveyard(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    if !q.obj(source).zone.is(Graveyard) {
        return None;
    }
    triggers::each_upkeep(q, source, event)
}

/// "If an opponent was dealt noncombat damage last turn" (CR 603.4): the turn before this one, an opponent of this card's
/// owner was dealt damage by something that was not combat.
fn opponent_was_dealt_noncombat_damage_last_turn(q: &dyn Query, asker: &Asker) -> bool {
    let last_turn = q.state().turn.number.saturating_sub(1);
    q.history().all().iter().any(|record| {
        record.turn == last_turn
            && matches!(record.event, Event::Damage { target: Target::Player(p), combat: false, .. } if p != asker.controller)
    })
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], command_the_stage),
    abilities: &[triggered(when(EventMask::STEP_BEGAN, each_upkeep_in_the_graveyard), effects::return_this_to_hand, "At the beginning of each upkeep, if an opponent was dealt noncombat damage last turn, return this card from your graveyard to your hand.")
        .anywhere()
        .condition(opponent_was_dealt_noncombat_damage_last_turn)],
    ..oracle::COMMAND_THE_STAGE
};
