use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, fight, lands, targets};
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "When you do, target Dinosaur you control deals damage equal to its power to another target creature",
    "Pay {cost}",
    "Don't pay",
);

/// Haste is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{2}"), &PAY),
        "When Itzquinth enters, you may pay {2}. When you do, target Dinosaur you control deals damage equal to its power to another target creature.",
    )],
    delayed: &[delayed_trigger(fight::first_target_bites_second, "When you do, target Dinosaur you control deals damage equal to its power to another target creature.").targets(&[
        target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Dinosaur) }>).labelled("Dinosaur you control to deal damage equal to its power"),
        targets::CREATURE_OTHER_THAN_THOSE_CHOSEN.labelled("Another creature to be dealt that damage"),
    ])],
    ..oracle::ITZQUINTH_FIRSTBORN_OF_GISHATH
};
