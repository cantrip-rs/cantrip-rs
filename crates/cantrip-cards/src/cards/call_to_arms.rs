use cantrip_api::*;
use cantrip_mechanics::{choose, choose_color_for, effects, statics};
use cantrip_oracle as oracle;

/// "As this enchantment enters, choose a color and an opponent. White creatures get +1/+1 as long as the chosen color is the most common color among nontoken permanents the chosen player controls but isn't tied for most common. When the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment." (CR 614.1c).
fn as_enchantment_enters_choose_color_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    entering.chosen_color = Some(choose_color_for(g, entering.controller, entering.object, ColorUse::Any)?);
    let opponents = g.state().opponents(entering.controller).collect();
    entering.chosen_player = choose::player(g, entering.controller, entering.object, opponents, "is the chosen player", 0)?;
    Ok(())
}

/// Whether the chosen color is the most common color among the nontoken permanents the chosen player controls, and not tied for
/// most common (CR 105.1, 109.5), as `source` chose them as it entered (CR 614.12a, 607.2d). `colors_of` says who controls a
/// permanent and what colors it is, as the caller reads them (through the layers, or as the game stands).
fn chosen_color_is_most_common(
    s: &GameState,
    link: Linked,
    colors_of: &dyn Fn(ObjectId) -> (PlayerId, ColorSet),
) -> bool {
    let (Some(player), Some(wanted)) = (s.chosen_player(link), s.chosen_color(link)) else { return false };
    let colors: Vec<ColorSet> = s.battlefield().iter().filter_map(|&id| {
        let (controller, colors) = colors_of(id);
        (controller == player && !s.obj(id).token).then_some(colors)
    }).collect();
    let count = |c: Color| colors.iter().filter(|set| set.has(c)).count();
    let best = Color::ALL.into_iter().map(count).max().unwrap_or(0);
    count(wanted) > 0 && count(wanted) == best && Color::ALL.into_iter().filter(|&c| count(c) == best).count() == 1
}

/// "White creatures get +1/+1 as long as the chosen color is the most common color among nontoken permanents the chosen player controls but isn't tied for most common." (CR 604.1).
fn white_creatures_get_1_1_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    so_far: &Chars,
) -> bool {
    chosen_color_is_most_common(q.state(), q.linked_to(source), &|id| (q.controller(id), q.chars(id).colors))
        && statics::creatures_of_color::<{ Color::White as u8 }>(q, source, affected, so_far)
}

/// "When the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment." (CR 603.8).
fn sacrifice_enchantment_state(q: &dyn Query, asker: &Asker) -> bool {
    !chosen_color_is_most_common(q.state(), q.linked_to(asker), &|id| (q.controller(id), q.chars(id).colors))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        state_triggered(
            sacrifice_enchantment_state,
            effects::sacrifice_self,
            "When the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            white_creatures_get_1_1_affects,
            statics::plus_pt::<1, 1>,
            "White creatures get +1/+1 as long as the chosen color is the most common color among nontoken permanents the chosen player controls but isn't tied for most common.",
        ),
    ],
    enters: Some(as_enchantment_enters_choose_color_enters),
    ..oracle::CALL_TO_ARMS
};
