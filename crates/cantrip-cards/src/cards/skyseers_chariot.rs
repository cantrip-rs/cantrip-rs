use cantrip_api::*;
use cantrip_mechanics::{crew, effects, library};
use cantrip_oracle as oracle;

/// "Activated abilities of sources with the chosen name cost {2} more to activate." (CR 602.2b, 601.2f): any source, whoever
/// controls it and from whatever zone it is activated, and its controller or not. There is no exception for mana abilities,
/// which are activated abilities (CR 605.1a) the core asks of too (CR 605.3). A spell is not an activated ability, nor is a
/// special action (CR 116.1), nor an ability granted to a player.
fn tax(q: &dyn Query, source: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Ability(object, ..) = costed else { return CostDelta::NONE };
    match q.state().chosen_card(q.text_linked(source)) {
        Some(chosen) if library::object_has_name(q, object, chosen) => CostDelta::more(2),
        _ => CostDelta::NONE,
    }
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::as_this_enters_choose_a_nonland_card_name),
    cost_changes: &[tax],
    abilities: &[crew::crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)")],
    ..oracle::SKYSEERS_CHARIOT
};
