use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("CreateToken", "pay {cost} to create two 1/1 white Dog creature tokens?", "", "Pay {cost}", "Don't pay");

/// "You may pay {1}{W}. If you do, create two 1/1 white Dog creature tokens" (CR 608.2d, 608.2c, 111.1): the payment is chosen as the
/// ability resolves, by the controller the Equipment last had (CR 603.10a), and the Dogs are made only if it was paid, two at once.
fn pay_for_two_dogs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{1}{W}"), &PAY)? {
        tokens::create(g, item.controller, &tokens::DOG, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}, {T}").and(SACRIFICE_THIS), effects::gain_three_life, "{2}, {T}, Sacrifice this Equipment: You gain 3 life."),
        triggered(
            triggers::THIS_DIES,
            pay_for_two_dogs,
            "When this Equipment is put into a graveyard from the battlefield, you may pay {1}{W}. If you do, create two 1/1 white Dog creature tokens.",
        ),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 0>, "Equipped creature gets +2/+0.")],
    ..oracle::KROVOD_HAUNCH
};
pub static EXTRA: &[&CardDef] = &[&tokens::DOG];
