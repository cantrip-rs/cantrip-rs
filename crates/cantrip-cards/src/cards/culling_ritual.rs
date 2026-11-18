use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Destroy each nonland permanent with mana value 2 or less. Add {B} or {G} for each permanent destroyed this way."
/// They are destroyed at once (CR 701.8a); one mana for each that was (not an indestructible one, nor one a regeneration
/// shield saved; a merged permanent once, CR 730.2), each of either color (ruling 2021-04-16: not limited to one color).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let destroyed = cantrip_mechanics::effects::destroy_nonland_permanents_with_mana_value_at_most(g, 2).count();
    if destroyed == 0 {
        return Ok(());
    }
    let choices = ColorSet::of(Color::Black).with(Color::Green);
    for color in cantrip_mechanics::choose_colors(g, item.controller, item.object, choices, destroyed as u32)? {
        g.add_mana(item.controller, color.mana(), 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ..oracle::CULLING_RITUAL
};
