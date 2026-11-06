use cantrip_api::*;
use cantrip_oracle as oracle;

/// As this creature enters, look at an opponent's hand, then choose any card name (CR 614.12a).
fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    cantrip_mechanics::hand::look_at_an_opponents_hand_then_choose_a_name(g, entering)
}

/// Spells your opponents cast with the chosen name cost {2} more to cast. Activated abilities of sources with the
/// chosen name cost {2} more to activate unless they're mana abilities (CR 601.2f, 602.2b): the core asks it of mana
/// abilities too (CR 605.3), and this card leaves them alone (CR 605.1a).
fn tax(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Some(chosen) = q.state().chosen_card(q.text_linked(source)) else { return CostDelta::NONE };
    let (object, spell) = match costed {
        Costed::Spell(id) => (id, true),
        Costed::Ability(_, ability, _) if q.activated_ability(ability).is_none_or(|d| d.is_mana_ability()) => return CostDelta::NONE,
        Costed::Ability(id, ..) => (id, false),
        Costed::PlayerAbility(..) => return CostDelta::NONE,
        // Plotting and the other special actions are no activated abilities (CR 116.1).
        Costed::SpecialAction(..) => return CostDelta::NONE,
    };
    if !cantrip_mechanics::library::object_has_name(q, object, chosen) || (spell && player == q.controller(source)) {
        return CostDelta::NONE;
    }
    CostDelta::more(2)
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters),
    cost_changes: &[tax],
    ..oracle::ANOINTED_PEACEKEEPER
};
