use cantrip_api::*;
use cantrip_mechanics::{earthbend, exile_and_return_transformed, greatest_power, statics, targets};
use cantrip_oracle as oracle;

/// I: "Draw cards equal to the greatest power among creatures you control" (CR 208.1, 121.1): counted as the chapter resolves; with
/// none, or none above 0, nothing is drawn.
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let greatest = greatest_power::among_creatures_you_control(g, item.controller);
    g.draw(item.controller, greatest);
    Ok(())
}

/// II: "Earthbend X, where X is the number of cards in your hand. That land becomes an Island in addition to its other types."
/// X is counted as the chapter resolves (CR 608.2h) and the land type is added with no duration (CR 305.7, 611.2a), in layer 4 so the
/// land keeps what it was and is a creature, an Island: it taps for {U} (CR 305.6). A land that is no longer a legal target is not
/// earthbent and no land becomes an Island (CR 608.2b).
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    let x = g.state().zone(Zone::hand(item.controller)).len() as i32;
    earthbend::earthbend(g, item, x)?;
    let island = g.words(item.object).subtype(Subtype::Island);
    g.add_effect(item.object, land, EffectKind::AddSubtype(island), Duration::Indefinite);
    Ok(())
}

/// III: "Exile this Saga, then return it to the battlefield transformed under your control": the ability's controller's, who need not own
/// it (CR 712.14a).
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_and_return_transformed(g, item.object, item.controller)
}

// --- Avatar Kyoshi ----------------------------------------------------------------------------------------------

/// "Lands you control have trample and hexproof" (CR 613.1f, layer 6): a land that is a creature - an earthbent one - attacks with
/// trample, and none can be targeted by an opponent.
fn have_trample_and_hexproof(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Trample).with(Keyword::Hexproof);
}

/// "{T}: Add X mana of any one color, where X is the greatest power among creatures you control" (CR 107.1b, 605.1a): counted as the
/// ability resolves, the layers' answer (CR 613), Avatar Kyoshi's own power included; with none above 0, the amount is 0 and no mana is added.
fn add_greatest_power_mana_of_one_color(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    let greatest = greatest_power::among_creatures_you_control(g, asker.controller);
    if let Some(color) = color { g.add_mana(asker.controller, color.mana(), greatest); }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add_by(add_greatest_power_mana_of_one_color).choose_color(),
        "{T}: Add X mana of any one color, where X is the greatest power among creatures you control.",
    )],
    statics: &[static_ability(Layer::Ability, statics::your_lands, have_trample_and_hexproof, "Lands you control have trample and hexproof.")],
    ..oracle::AVATAR_KYOSHI
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} Draw cards equal to the greatest power among creatures you control."),
        chapter(2, chapter_two, "II \u{2014} Earthbend X, where X is the number of cards in your hand. That land becomes an Island in addition to its other types.")
            .targets(&[earthbend::TARGET]),
        chapter(3, chapter_three, "III \u{2014} Exile this Saga, then return it to the battlefield transformed under your control."),
    ],
    delayed: &[earthbend::RETURN],
    back: Some(&BACK),
    ..oracle::THE_LEGEND_OF_KYOSHI
};
