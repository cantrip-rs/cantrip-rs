use cantrip_api::*;
use cantrip_mechanics::attach::equipment_controlled;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "This land enters tapped unless you control an Equipment" (CR 614.1d): read on the battlefield as it is while the land
/// enters.
fn enters_tapped_unless_equipment(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if equipment_controlled(g, entering.controller).is_empty() {
        entering.tapped = true;
    }
    Ok(())
}

/// "This ability costs {1} less to activate for each Equipment you control" (CR 601.2f): generic mana only, never the {R}.
fn cheaper_per_equipment(q: &dyn Query, asker: &Asker, _: Costed, _: &[Target]) -> CostDelta {
    let player = asker.controller;
    CostDelta::less(equipment_controlled(q, player).len().min(255) as u8)
}

fn make_dwarf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::DWARF, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_tapped_unless_equipment),
    abilities: &[ability(
        cost("{4}{R}, {T}"),
        make_dwarf,
        "{4}{R}, {T}: Create a 2/2 red Dwarf creature token. This ability costs {1} less to activate for each Equipment you control. Activate only as a sorcery.",
    )
    .sorcery_speed()
    .cost_change(cheaper_per_equipment)],
    ..oracle::THE_LONELY_MOUNTAIN
};

pub static EXTRA: &[&CardDef] = &[&tokens::DWARF];
