use cantrip_api::*;
use cantrip_mechanics::{text_change, cumulative_upkeep};
use cantrip_oracle as oracle;

/// "{T}: Change the text of target white enchantment you control that doesn't have cumulative upkeep by replacing all instances of one color word with another. (For example, you may change "black creatures can't attack" to "blue creatures can't attack.") That enchantment gains "Cumulative upkeep {1}." (At the beginning of its controller's upkeep, that player puts an age counter on it, then sacrifices it unless they pay its upkeep cost for each age counter on it.)" (CR 115.1).
fn change_text_target_white_enchantment_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).colors.has(q.words(asker.source).color(Color::White)) && q.chars(id)
        .is(CardType::Enchantment) && asker.controller == q.controller(id) && !q.has_keyword_family(id, KeywordFamily::CumulativeUpkeep)))
    )
}

/// "{T}: Change the text of target white enchantment you control that doesn't have cumulative upkeep by replacing all instances of one color word with another. (For example, you may change "black creatures can't attack" to "blue creatures can't attack.") That enchantment gains "Cumulative upkeep {1}." (At the beginning of its controller's upkeep, that player puts an age counter on it, then sacrifices it unless they pay its upkeep cost for each age counter on it.)" (CR 608.2, 611.2a).
fn change_text_target_white_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = cantrip_mechanics::targets::object(g, item, 0) else { return Ok(()) };
    let words = text_change::color_word(g, item.controller, item.object)?;
    text_change::change(g, item.object, object, words, Duration::Indefinite);
    g.end_sentence();
    g.add_effect(item.object, object, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0)), Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                change_text_target_white_enchantment,
                "{T}: Change the text of target white enchantment you control that doesn't have cumulative upkeep by replacing all instances of one color word with another. (For example, you may change \"black creatures can't attack\" to \"blue creatures can't attack.\") That enchantment gains \"Cumulative upkeep {1}.\" (At the beginning of its controller's upkeep, that player puts an age counter on it, then sacrifices it unless they pay its upkeep cost for each age counter on it.)",
            )
            .targets(&[target(change_text_target_white_enchantment_target_0)]),
    ],
    delayed: &[
        cumulative_upkeep!(cumulative_upkeep::Payment::Mana(mana("{1}")), "Cumulative upkeep {1}"),
    ],
    ..oracle::BALDUVIAN_SHAMAN
};
