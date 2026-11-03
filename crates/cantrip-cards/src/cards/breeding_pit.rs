use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice this enchantment unless you pay {B}{B}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_enchantment_unless_pay_b(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{B}{B}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

pub static THRULL: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet(4)),
    power: Some(0),
    toughness: Some(1),
    ..token!(Thrull)
};

/// "At the beginning of your end step, create a 0/1 black Thrull creature token." (CR 608.2).
fn create_0_1_black_thrull(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_4 = Vec::new();
    let def_4_0 = g.card_id(&THRULL);
    tokens_4.extend(std::iter::repeat((def_4_0, item.controller)).take(1));
    let words_4 = g.words(item.object);
    g.create_tokens_worded(&tokens_4, false, None, words_4)?;
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_enchantment_unless_pay_b,
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {B}{B}.",
        ),
        triggered(
            triggers::YOUR_END_STEP,
            create_0_1_black_thrull,
            "At the beginning of your end step, create a 0/1 black Thrull creature token.",
        ),
    ],
    ..oracle::BREEDING_PIT
};

pub static EXTRA: &[&CardDef] = &[&THRULL];
