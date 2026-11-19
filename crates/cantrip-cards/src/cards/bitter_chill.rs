use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_to_it, tap_enchanted},
    effects, targets,
    triggers::{THIS_DIES, THIS_ENTERS},
};
use cantrip_oracle as oracle;

const PAY_TO_SCRY: Question = Question::new("Pay", "pay {cost}?", "If you do, scry 1, then draw a card", "Pay {cost}", "Decline");

/// "You may pay {1}. If you do, scry 1, then draw a card." (CR 608.2d, 118.12): the Aura's controller is asked as it resolves.
fn may_pay_to_scry_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{1}"), &PAY_TO_SCRY)? {
        effects::scry_then_draw::<1>(g, item)?;
    }
    Ok(())
}

/// "Enchant creature. When this Aura enters, tap enchanted creature. Enchanted creature doesn't untap during its controller's
/// untap step. When this Aura is put into a graveyard from the battlefield, you may pay {1}. If you do, scry 1, then draw a card."
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature."),
        triggered(
            THIS_DIES,
            may_pay_to_scry_then_draw,
            "When this Aura is put into a graveyard from the battlefield, you may pay {1}. If you do, scry 1, then draw a card.",
        ),
    ],
    untap_bans: &[attached_to_it],
    ..oracle::BITTER_CHILL
};
