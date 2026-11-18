use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, face-down spells you cast this turn cost {1} less to cast" (CR 611.2c, 601.2f): the effect is
/// made for the player and lasts the turn whether or not the Maskmaker stays on the battlefield.
fn face_down_spells_cost_one_less_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.spells_cost_this_turn(item, item.controller, 0);
    Ok(())
}

/// A spell cast face down (CR 708.4, 702.37c, 702.168b) is face down as its total cost is determined (CR 601.2f); a face-up
/// one, or an ability, is not changed. The generic part only: a reduction never takes off a colored symbol (CR 118.7a).
fn face_down_spells_less(q: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(spell) if q.card(q.chars(spell).card).face_down => CostDelta::less(1),
        _ => CostDelta::NONE,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        face_down_spells_cost_one_less_this_turn,
        "Whenever this creature attacks, face-down spells you cast this turn cost {1} less to cast.",
    )],
    lasting_cost_changes: &[LastingCostChange { text: "Face-down spells you cast this turn cost {1} less to cast.", change: face_down_spells_less, next: None }],
    ..oracle::GOBLIN_MASKMAKER
};
