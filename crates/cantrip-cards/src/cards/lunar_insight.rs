use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Draw a card for each different mana value among nonland permanents you control" (CR 202.3, 121.1): the values are counted as the spell
/// resolves, each once however many permanents share it. A permanent with {X} in its cost has X as 0 (CR 202.3e), a token without a mana
/// cost a mana value of 0, a land is never counted.
fn draw_for_each_different_mana_value(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut values: Vec<u32> = Vec::new();
    for id in g.permanents_of(item.controller) {
        let value = g.mana_value(id);
        if !g.chars(id).is(CardType::Land) && !values.contains(&value) {
            values.push(value);
        }
    }
    effects::draw(g, item, values.len() as u32)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], draw_for_each_different_mana_value), ..oracle::LUNAR_INSIGHT };
