use cantrip_api::*;
use cantrip_mechanics::{statics, triggers};
use cantrip_oracle as oracle;

static PAY_QUESTION: Question = Question::new("Pay", "pay {cost}?", "", "Pay {cost}", "Decline");

/// The granted ability belongs to each creature: its controller pays, or that creature is destroyed (CR 118.12, 701.8).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid = g.may_pay(item.controller, item.object, item.object, cost("{1}"), &PAY_QUESTION)?;
    g.end_sentence();
    if !paid {
        g.destroy_all(&[item.object]);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        statics::all_creatures,
        statics::grant_delayed_ability,
        "All creatures have \"At the beginning of your upkeep, destroy this creature unless you pay {1}.\"",
    )],
    delayed: &[triggered(
        triggers::YOUR_UPKEEP,
        upkeep,
        "At the beginning of your upkeep, destroy this creature unless you pay {1}.",
    )],
    ..oracle::THE_TABERNACLE_AT_PENDRELL_VALE
};
