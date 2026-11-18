use cantrip_api::*;
use cantrip_mechanics::{choose_colors, history, targets};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, sacrifice a creature. Add X mana in any combination of {B} and/or {R}, where X is the sacrificed creature's mana value." (CR 608.2, 106.3).
fn as_additional_cost_cast_spell_2(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mana_2 = history::sacrificed_creatures_mana_value(g, item.object);
    if mana_2 > 0 {
        let colors = choose_colors(
            g,
            item.controller,
            item.object,
            ColorSet(12),
            mana_2,
        )?;
        for color in colors {
            g.add_mana(item.controller, color.mana(), 1);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], as_additional_cost_cast_spell_2),
    additional_cost: Some(sacrifice(1, targets::filters::creature)),
    ..oracle::BURNT_OFFERING
};
