use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// The 1/1 green Insect creature tokens it makes.
pub static INSECT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Insect)
};

pub static EXTRA: &[&CardDef] = &[&INSECT];

/// "Create that many 1/1 green Insect creature tokens": as many as the +1/+1 counters the cost removed (`StackItem::x`).
fn that_many_insects(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &INSECT, u32::from(item.x))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOU_DRAW_A_CARD, plus_one_counter_on_this, "Whenever you draw a card, put a +1/+1 counter on The Astonishing Ant-Man."),
        ability(
            Cost { variable: Some(VariableCost::RemoveAnyNumberOfCounters(CounterKind::PlusOne)), ..cost("{2}{G}, {T}") },
            that_many_insects,
            "{2}{G}, {T}, Remove any number of +1/+1 counters from The Astonishing Ant-Man: Create that many 1/1 green Insect creature tokens.",
        ),
    ],
    ..oracle::THE_ASTONISHING_ANT_MAN
};
