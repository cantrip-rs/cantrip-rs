use cantrip_api::*;
use cantrip_mechanics::{targets, COUNTERED_UNLESS_PAID};
use cantrip_oracle as oracle;

/// The spell's controller may pay either cost; otherwise the spell is countered (CR 118.12a, 701.6a).
fn counter_unless_paid(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let payer = g.controller(spell);
    if !g.may_pay(payer, item.object, spell, cost("{B}"), &PAY_BLACK)?
        && !g.may_pay(payer, item.object, spell, cost("{3}"), &COUNTERED_UNLESS_PAID)?
    {
        g.counter(spell);
    }
    Ok(())
}

static PAY_BLACK: Question = Question::new(
    "Pay",
    "pay {cost} to save {subject}, or consider paying {3} instead?",
    "",
    "Pay {cost}",
    "Consider {3}",
);

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{B}"),
        counter_unless_paid,
        "{1}{B}: Counter target black spell unless that spell's controller pays {B} or {3}.",
    ).targets(&[targets::spell_of_either_color::<{ Color::Black as u8 }, { Color::Black as u8 }>()])],
    ..oracle::THRULL_WIZARD
};
