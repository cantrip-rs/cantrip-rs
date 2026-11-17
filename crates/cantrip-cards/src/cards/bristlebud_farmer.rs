use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_ENTERS};
use cantrip_mechanics::{effects, library, select, targets, tokens};
use cantrip_oracle as oracle;

/// "When this creature enters, create two Food tokens" (CR 111.10b): made together (CR 111.1).
fn two_food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::FOOD, 2)?;
    Ok(())
}

/// "A Food" (CR 205.3g): a permanent with the Food subtype, a token or not.
const FOOD: select::Selection = select::Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Food)]).called("Food");

/// "You may sacrifice a Food. If you do, mill three cards. You may put a permanent card from among them into your hand."
/// (CR 701.21a, 608.2c, 701.17): the sacrifice is chosen as the ability resolves, not paid as a cost, and the mill happens only
/// if a Food really was sacrificed. A Food is any permanent with the Food subtype, a token or not; nothing is asked with none.
fn sacrifice_a_food_to_mill(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, FOOD)?.is_empty() {
        library::mill_then_may_take_into_hand(g, item.controller, item.object, 3, "permanent", targets::is_permanent_card)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // Trample is a core keyword.
    abilities: &[
        triggered(THIS_ENTERS, two_food, "When this creature enters, create two Food tokens. (They're artifacts with \"{2}, {T}, Sacrifice this token: You gain 3 life.\")"),
        triggered(
            THIS_ATTACKS,
            sacrifice_a_food_to_mill,
            "Whenever this creature attacks, you may sacrifice a Food. If you do, mill three cards. You may put a permanent card from among them into your hand.",
        ),
    ],
    ..oracle::BRISTLEBUD_FARMER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
