use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::{DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT, PROWESS};
use cantrip_oracle as oracle;

/// "Whenever this creature deals combat damage to a player, put that many incubation counters on it" (CR 510.2,
/// 603.2): the damage it dealt rides on the trigger (`data[0]`), so two hits are two triggers, each with its own
/// amount. The counters go on it if it is still on the battlefield (CR 400.7).
fn incubate_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if targets::permanent(g, item.object) {
        g.add_counters(item.object, CounterKind::Incubation, item.data[0] as i16);
    }
    Ok(())
}

/// "Create a 2/2 blue Drake creature token with flying."
fn hatch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &DRAKE, 1)?;
    Ok(())
}

/// The 2/2 blue Drake creature token with flying that Drake Hatcher makes.
static DRAKE: CardDef = CardDef {
    name: "Drake Token",
    token_name: Some(&[Subtype::Drake]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Drake]),
    power: Some(2),
    toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        PROWESS,
        triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT, incubate_damage, "Whenever this creature deals combat damage to a player, put that many incubation counters on it."),
        ability(remove_counters(CounterKind::Incubation, 3), hatch, "Remove three incubation counters from this creature: Create a 2/2 blue Drake creature token with flying."),
    ],
    ..oracle::DRAKE_HATCHER
};

pub static EXTRA: &[&CardDef] = &[&DRAKE];
