use cantrip_api::*;
use cantrip_mechanics::{choose_color, history, targets};
use cantrip_oracle as oracle;

/// "Add X mana of any one color, where X is 1 plus the sacrificed creature's mana value. Spend this mana only to cast creature spells." (CR 608.2h, 106.3).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount = 1 + history::sacrificed_creatures_mana_value(g, item.object);
    let color = choose_color(g, item.controller, item.object)?;
    g.add_restricted_mana(item.controller, color.mana(), amount, ManaRestriction::CREATURE_SPELLS);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], resolve),
    additional_cost: Some(sacrifice(1, targets::filters::creature)),
    ..oracle::METAMORPHOSIS
};
