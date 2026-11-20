use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_mechanics::{library, select, statics};
use cantrip_oracle as oracle;

/// "Mill three cards, then return a land card from your graveyard to the battlefield tapped."
fn mill_three_then_return_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_return_a_card_to_the_battlefield_tapped(g, item, 3, select::land_cards(select::as_many_as_can(1)))
}

/// "Activated abilities of lands you control cost {1} less to activate" (CR 601.2f, 602.2b, 118.7a, 605.1a): any activated ability, a
/// mana ability included (the core asks of those too, CR 605.3), of a land on the battlefield under this permanent's controller's
/// control, whoever activates it and whatever it costs: generic mana only comes off, and nothing is taken from a cost with none. An
/// ability a land card has in another zone (cycling) is not reduced (the ruling), nor is a special action (CR 116.1): `Costed::Ability` alone.
fn land_abilities_cost_one_less(q: &dyn Query, source: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Ability(land, ..) = costed else { return CostDelta::NONE };
    if q.is_on_battlefield(land) && q.controller(land) == q.controller(source) && q.chars(land).is(CardType::Land) {
        CostDelta::less(1)
    } else {
        CostDelta::NONE
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_OR_ATTACKS,
        mill_three_then_return_a_land,
        "Whenever this creature enters or attacks, mill three cards, then return a land card from your graveyard to the battlefield tapped.",
    )],
    cost_changes: &[land_abilities_cost_one_less],
    statics: &[static_ability(Layer::PtModify, statics::your_land_creatures, statics::plus_pt::<1, 1>, "Land creatures you control get +1/+1.")],
    ..oracle::BLOSSOMING_TORTOISE
};
