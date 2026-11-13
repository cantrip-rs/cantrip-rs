use cantrip_api::select::{part, Choice, SelectFn, Verdict};
use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{library, select, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "A creature or Vehicle card" (CR 205.3g): a card in a graveyard with the creature type or the Vehicle subtype.
fn creature_or_vehicle_card(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| targets::is_creature_or_vehicle(&q.chars(card)))
}

const CREATURE_OR_VEHICLE_CARD: select::Selection =
    select::Selection::of(select::as_many_as_can(1), &[part(&(creature_or_vehicle_card as SelectFn), 0, "")]).called("creature or Vehicle");

/// "Mill two cards, then return a creature or Vehicle card from your graveyard to your hand."
fn mill_then_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_return_a_card(g, item, 2, CREATURE_OR_VEHICLE_CARD)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, mill_then_return, "When this Vehicle enters, mill two cards, then return a creature or Vehicle card from your graveyard to your hand."),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::CARRION_CRUISER
};
