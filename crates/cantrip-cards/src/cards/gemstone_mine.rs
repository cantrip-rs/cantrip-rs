use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Add one mana of any color. If there are no mining counters on this land, sacrifice it (CR 605.3a): as the ability
/// resolves, so the mana is added either way.
fn add_one_of_any_color(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, color.unwrap_or(Color::White).mana(), 1);
    let land = asker.source;
    if g.is_on_battlefield(land) && g.obj(land).counters(CounterKind::Mining) == 0 {
        g.sacrifice(asker.controller, &[land]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // This land enters with three mining counters on it (CR 614.1c, 122.6).
    enters: Some(effects::enters_with_counters::<{ CounterKind::Mining as usize }, 3>),
    abilities: &[mana_ability(TAP.and(remove_counters(CounterKind::Mining, 1)), add_any_color(1).by(add_one_of_any_color), "{T}, Remove a mining counter from this land: Add one mana of any color. If there are no mining counters on this land, sacrifice it.")],
    ..oracle::GEMSTONE_MINE
};
