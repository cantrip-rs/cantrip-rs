use cantrip_api::*;
use cantrip_mechanics::{each_opponent_sacrifices, effects};
use cantrip_oracle as oracle;

/// IV: "Each opponent sacrifices a creature of their choice and loses 3 life." (CR 701.21a, 119.3, 608.2c): each opponent chooses
/// and sacrifices a creature they control (nothing, with none), then all of them lose 3 life; the life is lost whether or not a
/// creature was sacrificed.
fn oblivion(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_opponent_sacrifices(g, item.controller, item.object, 1, |q, id| q.chars(id).is(CardType::Creature))?;
    g.each_opponent_loses_life(item.controller, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::draw_a_card_and_lose_1_life, "I \u{2014} Pain \u{2014} You draw a card and you lose 1 life."),
        chapter(2, effects::draw_a_card_and_lose_1_life, "II \u{2014} Pain \u{2014} You draw a card and you lose 1 life."),
        chapter(3, effects::draw_a_card_and_lose_1_life, "III \u{2014} Pain \u{2014} You draw a card and you lose 1 life."),
        chapter(4, oblivion, "IV \u{2014} Oblivion \u{2014} Each opponent sacrifices a creature of their choice and loses 3 life."),
    ],
    ..oracle::SUMMON_ANIMA
};
