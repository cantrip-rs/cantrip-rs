use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// Target opponent reveals their hand. You may choose a nonland card from it; if you do, that player discards it. If you
/// don't (no nonland card, an empty hand or a choice declined), put two +1/+1 counters on a creature or Vehicle you
/// control, chosen as the spell resolves (CR 608.2d): a lone one is taken without asking.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = item.target(0).player() else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(opponent)).to_vec();
    let mut chosen: Vec<ObjectId> = Vec::new();
    if !hand.is_empty() {
        g.reveal(opponent, &hand);
        chosen = choose::select(g, item.controller, item.object, Purpose::Discard, hand, select::NONLAND_CARD)?;
    }
    if !chosen.is_empty() {
        g.discard_revealed(&chosen);
        return Ok(());
    }
    let candidates: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| targets::creature_or_vehicle(g, id))
        .collect();
    let creature = match candidates.len() {
        0 => None,
        1 => Some(candidates[0]),
        _ => {
            let ask = RequestKind::ChooseTarget { replacing: None, label: "", slot_player: None, together: None,
                source: item.object,
                purpose: Purpose::Targets,
                candidates: candidates.iter().map(|&id| Target::Object(id)).collect(),
                optional: false,
                least: 1,
                count: 1,
                any_number: false,
            };
            match g.ask(item.controller, ask)? {
                Answer::Targets(t) => t.first().and_then(|t| t.object()),
                _ => None,
            }
        }
    };
    if let Some(creature) = creature {
        g.add_counters(creature, CounterKind::PlusOne, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], effect),
    ..oracle::RECKONER_SHAKEDOWN
};
