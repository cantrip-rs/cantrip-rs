use cantrip_api::*;
use cantrip_mechanics::select::{self, Selection};
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_ENTERS};
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Create half X Food tokens, rounded up" (CR 107.1a, 111.10b): the X it was cast with (CR 107.3m), read from the creature even if
/// it has left the battlefield by the time the ability resolves (last-known information, CR 608.2h), made together (CR 101.4). Put
/// onto the battlefield without being cast, X is 0 and none are made.
fn create_half_x_foods(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = u32::from(g.obj(item.object).x);
    tokens::create(g, item.controller, &tokens::FOOD, x.div_ceil(2))?;
    Ok(())
}

const A_FOOD: Selection = Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Food)]).called("Food");

/// "You may sacrifice a Food. If you do, draw a card." (CR 701.21a, 608.2c-d): the Food is chosen as the ability resolves, a Food
/// token or a Food artifact, not paid as a cost; with none to sacrifice nothing is asked and nothing is drawn.
fn may_sacrifice_a_food_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, A_FOOD)?.is_empty() {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::PlusOne as usize }>),
    abilities: &[
        triggered(THIS_ENTERS, create_half_x_foods, "When The Goose Mother enters, create half X Food tokens, rounded up."),
        triggered(THIS_ATTACKS, may_sacrifice_a_food_to_draw, "Whenever The Goose Mother attacks, you may sacrifice a Food. If you do, draw a card."),
    ],
    ..oracle::THE_GOOSE_MOTHER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
