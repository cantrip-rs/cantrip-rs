use cantrip_api::*;
use cantrip_mechanics::{effects, hand};
use cantrip_oracle as oracle;

/// "II \u{2014} You may put a Hero creature card with mana value 3 or less from your hand onto the battlefield. If you don't,
/// draw a card." The eligible cards are read as the chapter resolves (CR 608.2d): creature cards in hand with the creature
/// type Hero as this Saga's text says it (CR 205.3m, 612) and mana value 3 or less (CR 202.3). If none is eligible, or none is
/// chosen, you didn't, and you draw a card (CR 608.2c, 121.1); nor did you if a replacement sent the card elsewhere instead
/// (Containment Priest's exile): the event it replaced never happened (CR 614.6).
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let hero = g.words(item.object).subtype(Subtype::Hero);
    let eligible: Vec<CardId> = g
        .state()
        .zone(Zone::hand(you))
        .to_vec()
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.chars(id).has_subtype(hero) && g.mana_value(id) <= 3)
        .map(|id| g.chars(id).card)
        .collect();
    let put = hand::may_put_onto_battlefield(g, you, item.object, |c| eligible.contains(&c.card))?;
    if !put.is_some_and(|hero| g.is_on_battlefield(hero)) {
        g.draw(you, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, |g, item| effects::scry(g, item, 2), "I \u{2014} Scry 2."),
        chapter(
            2,
            chapter_two,
            "II \u{2014} You may put a Hero creature card with mana value 3 or less from your hand onto the battlefield. If you don't, draw a card.",
        ),
        chapter(3, effects::plus_one_counter_on_each_creature_you_control, "III \u{2014} Put a +1/+1 counter on each creature you control."),
    ],
    ..oracle::ORIGIN_OF_THE_AVENGERS
};
