use cantrip_api::*;
use cantrip_mechanics::{statics, triggers};
use cantrip_oracle as oracle;

/// "All artifacts have "At the beginning of your upkeep, sacrifice this artifact unless you pay {2}."" (CR 604.1).
fn all_artifacts_have_beginning_upkeep_affects(
    _: &dyn LayerQuery,
    _: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    (*so_far).is(CardType::Artifact)
}

/// "At the beginning of your upkeep, sacrifice this artifact unless you pay {2}." (CR 608.2, 701.21a, 118.1).
fn granted_0(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{2}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost} to avoid sacrificing this artifact?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            all_artifacts_have_beginning_upkeep_affects,
            statics::grant_delayed_ability,
            "All artifacts have \"At the beginning of your upkeep, sacrifice this artifact unless you pay {2}.\"",
        ),
    ],
    delayed: &[
        triggered(
            triggers::YOUR_UPKEEP,
            granted_0,
            "At the beginning of your upkeep, sacrifice this artifact unless you pay {2}.",
        ),
    ],
    ..oracle::ENERGY_FLUX
};
