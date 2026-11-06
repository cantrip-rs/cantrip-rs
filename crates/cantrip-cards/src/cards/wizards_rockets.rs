use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "Add X mana in any combination of colors" (CR 106.1b): X was announced with the activation (CR 107.3a, 605.3a) and
/// is local to that activation, even after its source is sacrificed. Each mana is a color chosen as the ability
/// resolves. Questions use the wire's color-count width in chunks, without reducing the amount of mana produced.
fn add_x_mana(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let mut left = g.mana_ability_x().unwrap_or(0);
    while left > 0 {
        let count = left.min(u32::from(u8::MAX)) as u8;
        for color in cantrip_mechanics::choose_colors(g, asker.controller, asker.source, ColorSet(0b11111), count)? {
            g.add_mana(asker.controller, color.mana(), 1);
        }
        left -= u32::from(count);
    }
    Ok(())
}

/// "When this artifact is put into a graveyard from the battlefield" (CR 700.4, 603.10a): looked back on.
pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(cost("{X}, {T}").and(SACRIFICE_THIS), add_by(add_x_mana), "{X}, {T}, Sacrifice this artifact: Add X mana in any combination of colors.").x_is_mana_amount(),
        triggered(THIS_DIES, |g, item| effects::draw(g, item, 1), "When this artifact is put into a graveyard from the battlefield, draw a card."),
    ],
    enters: Some(cantrip_mechanics::mana::enters_tapped),
    ..oracle::WIZARDS_ROCKETS
};
