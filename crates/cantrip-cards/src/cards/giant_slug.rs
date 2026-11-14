use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// Choose a basic land type and gain its landwalk through cleanup (CR 603.7e, 702.14a).
fn delayed_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // These are the five basic land types, not words in the Slug's text that a text change can alter.
    let options = (0..5).map(|i| Label::WordedOption(item.ability.card, i, Words::NONE)).collect();
    let chosen = choose::one_of(g, item.controller, item.object, options)?;
    let keyword = Keyword::LANDWALKS[usize::from(chosen)].0;
    g.add_effect(item.object, item.object, EffectKind::AddKeywords(Keywords::of(&[keyword])), Duration::EndOfTurn);
    Ok(())
}

/// "{5}: At the beginning of your next upkeep, choose a basic land type. This creature gains landwalk of the chosen type until the end of that turn." (CR 608.2).
fn beginning_next_upkeep_choose_basic(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.delayed_trigger(
        DelayedWhen::Step {
            step: Step::Upkeep,
            on_turn_of: Some(item.controller),
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{5}"),
            beginning_next_upkeep_choose_basic,
            "{5}: At the beginning of your next upkeep, choose a basic land type. This creature gains landwalk of the chosen type until the end of that turn.",
        ),
    ],
    delayed: &[
        delayed_trigger(
            delayed_1,
            "At the beginning of your next upkeep, choose a basic land type.",
        ),
    ],
    labels: &["Plains", "Island", "Swamp", "Mountain", "Forest"],
    ..oracle::GIANT_SLUG
};
