use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets, triggers};
use cantrip_oracle as oracle;

/// The trigger's controller may pay; otherwise they sacrifice this Aura if they still control it (CR 118.12a, 701.21a).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{1}{U}"), &PAY)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

static PAY: Question = Question::new("Pay", "pay {cost}, or sacrifice {subject}?", "", "Pay {cost}", "Don't pay");

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        upkeep,
        "At the beginning of your upkeep, sacrifice this Aura unless you pay {1}{U}.",
    )],
    statics: &[
        control_changing(attach::attached_object, "You control enchanted creature."),
        static_ability(Layer::PtModify, attach::attached_object, statics::plus_pt_saturating::<0, 1>, "Enchanted creature gets +0/+1."),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::BINDING_GRASP
};
