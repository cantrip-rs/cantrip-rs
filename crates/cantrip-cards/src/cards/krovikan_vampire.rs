use cantrip_api::*;
use cantrip_mechanics::{delayed, triggers};
use cantrip_oracle as oracle;

/// Creatures this object damaged which subsequently died this turn, as battlefield objects (CR 700.4, 603.4).
fn deaths(q: &dyn Query, source: ObjectId) -> Vec<ObjectId> {
    let mut damaged = std::collections::HashSet::new();
    let mut died = Vec::new();
    for r in q.history().this_turn() {
        match r.event {
            Event::Damage { source: from, target: Target::Object(target), amount, .. }
                if from == source && amount > 0 => { damaged.insert(target); }
            Event::ZoneChange { old, from, to, part: false, .. }
                if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard)
                    && damaged.contains(&old) && q.chars(old).is(CardType::Creature) => died.push(old),
            _ => {}
        }
    }
    died
}

/// The intervening if asks whether a creature died, even if the card later left its graveyard (CR 603.4).
fn a_damaged_creature_died(q: &dyn Query, asker: &Asker) -> bool {
    !deaths(q, asker.source).is_empty()
}

/// Return each first graveyard incarnation still there, including components of a merged permanent (CR 730.3c).
fn return_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut cards = Vec::new();
    for old in deaths(g, item.object) {
        for card in g.found_all_after_move(old) {
            if g.obj(card).zone.is(ZoneKind::Graveyard) && !g.obj(card).token && !g.obj(card).card_copy {
                cards.push((card, item.controller, false));
            }
        }
    }
    let returned: Vec<_> = g.put_all_onto_battlefield_as(&cards)?.into_iter()
        .filter(|&id| is_live(g, id) && g.obj(id).zone.is(ZoneKind::Battlefield)).collect();
    g.end_sentence();
    if !returned.is_empty() {
        g.delayed_trigger_about(DelayedWhen::WhenEvent, item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), &returned);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::EACH_END_STEP, return_cards, "At the beginning of each end step, if a creature dealt damage by this creature this turn died, put that card onto the battlefield under your control. Sacrifice it when you lose control of this creature.")
        .condition(a_damaged_creature_died)],
    delayed: &[triggered(triggers::YOU_LOSE_CONTROL_OF_THIS, delayed::sacrifice_created, "Sacrifice it when you lose control of this creature.")],
    ..oracle::KROVIKAN_VAMPIRE
};
