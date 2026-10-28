use cantrip_api::*;
use cantrip_mechanics::amass::amass;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "You draw a card and lose 1 life. Amass Goblins 2." (CR 701.47a): two sentences, so the life loss is done before the Army
/// is made or grown (CR 608.2c).
fn draw_and_amass(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw_a_card_and_lose_1_life(g, item)?;
    g.end_sentence();
    amass(g, item.controller, item.object, &tokens::GOBLIN_ARMY, Subtype::Goblin, 2)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], draw_and_amass), ..oracle::RAGE_INTO_THE_VALLEY };

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];
