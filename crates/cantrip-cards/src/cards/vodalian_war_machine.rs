use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::{behold, effects, triggers};

/// An untapped Merfolk can pay either ability (CR 118.3).
fn untapped_merfolk(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    !q.state().obj(id).tapped && behold::merfolk(q, asker, id)
}

/// The permanents tapped this turn to pay the cost of an activation of `source` (CR 602.2b): the history records each object a
/// cost paid, before the ability's activation, under the activation's serial.
fn paid_for_activations_of(history: &History, source: ObjectId) -> Vec<ObjectId> {
    let mut paying = None;
    let mut paid = Vec::new();
    for r in history.this_turn() {
        match r.event {
            Event::AbilityCostObjectPaid { serial, .. } => paying = Some(serial),
            Event::AbilityActivated { source: activated, .. } => {
                if let (true, Some(serial)) = (activated == source, paying) {
                    paid.extend(history.ability_cost_objects_of_part(serial, CostPart::Permanents));
                }
                paying = None;
            }
            _ => {}
        }
    }
    paid
}

/// "When this creature dies, destroy all Merfolk tapped this turn to pay for its abilities." (CR 701.8a).
fn destroy_all_merfolk_tapped_pay(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let merfolk = {
        let q: &dyn Query = g;
        let paid = paid_for_activations_of(&q.history(), item.object);
        let word = q.words(item.object).subtype(Subtype::Merfolk);
        q.state().battlefield().iter().copied().filter(|id| paid.contains(id) && q.chars(*id).has_subtype(word)).collect::<Vec<_>>()
    };
    g.destroy_all(&merfolk);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            tap_untapped(1, untapped_merfolk),
            effects::this_can_attack_as_though_it_didnt_have_defender,
            "Tap an untapped Merfolk you control: This creature can attack this turn as though it didn't have defender.",
        ),
        ability(
            tap_untapped(1, untapped_merfolk),
            effects::pump_self::<2, 1>,
            "Tap an untapped Merfolk you control: This creature gets +2/+1 until end of turn.",
        ),
        triggered(
            triggers::THIS_DIES,
            destroy_all_merfolk_tapped_pay,
            "When this creature dies, destroy all Merfolk tapped this turn to pay for its abilities.",
        ),
    ],
    ..oracle::VODALIAN_WAR_MACHINE
};
