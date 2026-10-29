use cantrip_api::*;
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

/// The number of differently named lands `player` controls, read during the layer pass: each land's names through the
/// layers (`library::object_names_in_layers`) and the largest set of them with no name in common (CR 201.2a-b), as
/// `lands::differently_named_lands` counts outside the layers.
fn differently_named_lands_in_layers(q: &dyn LayerQuery, player: PlayerId) -> usize {
    let names: Vec<Vec<&'static str>> = q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| q.controller(id) == player && q.chars(id).is(CardType::Land))
        .map(|id| library::object_names_in_layers(q, id))
        .collect();
    select::largest_name_set(&names).len()
}

/// "Captain Pike's power and toughness are each equal to the number of differently named lands you control." (CR 604.3,
/// 613.4a): characteristic-defining, so it applies in every zone and reads current land names through the layers.
fn size(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let controller = q.controller(source);
    let count = differently_named_lands_in_layers(q, controller) as i16;
    chars.power = Some(count);
    chars.toughness = Some(count);
}

/// The two permissions use the same game rules as Ka-Zar and Traveling Chocobo.
pub static CARD: CardDef = CardDef {
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopLands],
    statics: &[characteristic_defining(
        Layer::PtDefine,
        size,
        "Captain Pike's power and toughness are each equal to the number of differently named lands you control.",
    )
    .lost_pt(0, 0)],
    ..oracle::CAPTAIN_PIKE_FULLNESS_OF_TIME
};
