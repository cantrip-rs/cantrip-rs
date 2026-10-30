use cantrip_api::*;
use cantrip_mechanics::{attach::{attached_to_it, tap_attached}, targets, triggers};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to tap enchanted permanent?", "It still won't untap during its controller's untap step if you decline", "Pay {cost}", "Don't pay");

/// "When this Aura enters, you may pay {3}. If you do, tap enchanted permanent" (CR 118.12, 303.4m).
fn may_pay_to_tap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{3}"), &PAY)? {
        tap_attached(g, item);
    }
    Ok(())
}

/// "Enchanted permanent ... can't be beamed up" (CR 101.2, 118.3): only the beam me up cost,
/// not other returns to hand. The restriction belongs to the Aura, not the enchanted permanent's abilities.
fn cant_be_beamed_up(q: &dyn Query, me: &Asker, _: PlayerId, _: CostFor, payment: CostPayment) -> bool {
    matches!(payment, CostPayment::BeamUp(permanent) if attached_to_it(q, me, permanent))
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::ARTIFACT_OR_CREATURE),
    abilities: &[triggered(triggers::THIS_ENTERS, may_pay_to_tap,
        "When this Aura enters, you may pay {3}. If you do, tap enchanted permanent.")],
    untap_bans: &[attached_to_it],
    cost_bans: &[cant_be_beamed_up],
    ..oracle::MAROONED
};
