use cantrip_api::*;
use cantrip_mechanics::attach::equipment_controlled;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// The 2/2 red Dwarf creature token it makes.
pub static DWARF: CardDef = CardDef {
    name: "Dwarf Token",
    token_name: Some(&[Subtype::Dwarf]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Dwarf]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// "This land enters tapped unless you control an Equipment" (CR 614.1c): read on the battlefield as it is while the land
/// enters.
fn enters_tapped_unless_equipment(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if equipment_controlled(g, entering.controller).is_empty() {
        entering.tapped = true;
    }
    Ok(())
}

/// "This ability costs {1} less to activate for each Equipment you control" (CR 601.2f): generic mana only, never the {R}.
fn cheaper_per_equipment(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(equipment_controlled(q, player).len().min(255) as u8)
}

fn make_dwarf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &DWARF, 1)?;
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

pub static EXTRA: &[&CardDef] = &[&DWARF];
