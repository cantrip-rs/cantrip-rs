use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// I: "Mill three cards, then create a Food token." (CR 701.17a, 608.2c): the Food is made after the cards are milled.
fn mill_three_then_food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 3);
    tokens::create_a_food_token(g, item)
}

/// III: "Draw a card if there's a creature or Lesson card in your graveyard." (CR 205.3k): checked as the chapter resolves, after
/// any card that the earlier chapters milled; an opponent's graveyard does not count.
fn draw_if_creature_or_lesson_in_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let any = g.state().zone(Zone::graveyard(you)).iter().any(|&card| g.chars(card).is(CardType::Creature) || g.chars(card).has_subtype(Subtype::Lesson));
    if any {
        g.draw(you, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, mill_three_then_food, "I \u{2014} Mill three cards, then create a Food token."),
        chapter(2, effects::plus_one_counter_on_each_target, "II \u{2014} Put a +1/+1 counter on each of up to two target creatures you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL.between(0, 2)]),
        chapter(3, draw_if_creature_or_lesson_in_graveyard, "III \u{2014} Draw a card if there's a creature or Lesson card in your graveyard."),
    ],
    ..oracle::LEAVES_FROM_THE_VINE
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
