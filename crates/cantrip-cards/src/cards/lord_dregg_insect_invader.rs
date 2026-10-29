use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{history, targets, tokens};
use cantrip_oracle as oracle;

/// The 1/1 black Insect Warrior creature token with flying.
pub static INSECT_WARRIOR: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Insect Warrior)
};

fn create_an_insect_warrior(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &INSECT_WARRIOR, 1)?;
    Ok(())
}

fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            YOUR_END_STEP,
            create_an_insect_warrior,
            "Disappear \u{2014} At the beginning of your end step, if a permanent left the battlefield under your control this turn, create a 1/1 black Insect Warrior creature token with flying.",
        )
        .condition(history::disappear),
        ability(cost("{3}{G}").and(sacrifice(1, targets::token_you_control)), draw_a_card, "{3}{G}, Sacrifice a token: Draw a card."),
    ],
    ..oracle::LORD_DREGG_INSECT_INVADER
};

pub static EXTRA: &[&CardDef] = &[&INSECT_WARRIOR];
