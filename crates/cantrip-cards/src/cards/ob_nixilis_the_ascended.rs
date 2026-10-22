use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::{EACH_END_STEP, THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Destroy all tapped creatures your opponents control. You gain 1 life for each creature destroyed this way." One
/// destroy event (CR 701.8) and one life gain for the number that really left the battlefield (CR 119.3): an indestructible
/// creature or a regeneration shield saves its own, and buys no life. Nothing destroyed, nothing gained.
fn destroy_tapped_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let opponents: Vec<PlayerId> = g.state().opponents(you).collect();
    let tapped: Vec<ObjectId> = opponents
        .iter()
        .flat_map(|&player| g.permanents_of(player))
        .filter(|&id| g.obj(id).tapped && g.chars(id).is(CardType::Creature))
        .collect();
    let destroyed = g.destroy_all(&tapped);
    if !destroyed.is_empty() {
        g.gain_life(you, destroyed.len() as i32);
    }
    Ok(())
}

/// "Create a 4/4 white Angel creature token with flying."
fn create_angel(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ANGEL, 1)?;
    Ok(())
}

/// The 4/4 white Angel creature token with flying that Ob Nixilis makes (CR 111.10f): no vigilance, unlike Resplendent
/// Angel's.
pub static ANGEL: CardDef = CardDef {
    name: "Angel Token",
    token_name: Some(&[Subtype::Angel]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Angel]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..CardDef::BLANK
};

pub static EXTRA: &[&CardDef] = &[&ANGEL];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            destroy_tapped_creatures,
            "When Ob Nixilis enters, destroy all tapped creatures your opponents control. You gain 1 life for each creature destroyed this way.",
        ),
        triggered(
            EACH_END_STEP,
            create_angel,
            "At the beginning of each end step, if you gained life this turn, create a 4/4 white Angel creature token with flying.",
        )
        .condition(history::you_gained_life_this_turn),
    ],
    ..oracle::OB_NIXILIS_THE_ASCENDED
};
