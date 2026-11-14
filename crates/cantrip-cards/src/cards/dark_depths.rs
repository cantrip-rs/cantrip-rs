use cantrip_api::*;
use cantrip_oracle as oracle;

/// Marit Lage, a legendary 20/20 black Avatar creature token with flying and indestructible (CR 111.4, 205.4a): named by the
/// effect that makes it.
pub static MARIT_LAGE: CardDef = CardDef {
    name: "Marit Lage",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Avatar]),
    power: Some(20),
    toughness: Some(20),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Indestructible]),
    ..CardDef::BLANK
};

/// "Dark Depths enters with ten ice counters on it" (CR 614.1c, 122.6). A replacement of how it enters, so a permanent
/// entering as a copy of it has it too (CR 707.9a: Vesuva), and one that becomes a copy of it on the battlefield does not
/// (Thespian's Stage's ruling).
fn enters_with_ten_ice(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::Ice as usize] += 10;
    Ok(())
}

/// "{3}: Remove an ice counter from Dark Depths." The removal is the effect, not a cost: with none on it, nothing happens.
fn remove_ice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.remove_counters(item.object, CounterKind::Ice, 1);
    Ok(())
}

/// "When Dark Depths has no ice counters on it": a state trigger (CR 603.8), true of Dark Depths on the battlefield. It
/// triggers once and not again while it is on the stack (the ruling).
fn no_ice(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).counters(CounterKind::Ice) == 0
}

/// "... sacrifice it. If you do, create Marit Lage ..." Only the Dark Depths that triggered can be sacrificed: if it has
/// left the battlefield it is a new object (CR 400.7), nothing is sacrificed and no Marit Lage is made (the ruling). "If you
/// do" is that it was sacrificed (CR 608.2c).
fn sacrifice_for_marit_lage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.sacrifice(item.controller, &[item.object]).is_empty() {
        cantrip_mechanics::tokens::create(g, item.controller, &MARIT_LAGE, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_ten_ice),
    abilities: &[
        ability(cost("{3}"), remove_ice, "{3}: Remove an ice counter from Dark Depths."),
        state_triggered(no_ice, sacrifice_for_marit_lage, "When Dark Depths has no ice counters on it, sacrifice it. If you do, create Marit Lage, a legendary 20/20 black Avatar creature token with flying and indestructible."),
    ],
    ..oracle::DARK_DEPTHS
};

pub static EXTRA: &[&CardDef] = &[&MARIT_LAGE];
