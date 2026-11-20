use cantrip_api::*;
use cantrip_mechanics::{same_name, targets};
use cantrip_oracle as oracle;

/// "Target nontoken creature you control that doesn't have the same name as a token you control" (CR 115.1, 201.2, 608.2b): a
/// creature on the battlefield under the ability's controller that is no token, and that no token they control shares a name with,
/// by the names they have now (`same_name`: a token copy has the name it copies). It is judged again as the ability resolves.
fn nontoken_creature_you_control_with_no_token_of_its_name(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id)
        if targets::permanent_you_control_filter(q, asker, t)
            && q.chars(id).is(CardType::Creature)
            && !q.obj(id).token
            && !q.permanents_of(asker.controller).into_iter().any(|other| q.obj(other).token && same_name(q, other, id)))
}

/// I, II: "Create a token that's a copy of it, except it isn't legendary, is a Reflection in addition to its other types, and has
/// haste." (CR 707.2, 707.9b, 111.1, 608.2b): all three exceptions are part of the copy's copiable values, so a copy of the token
/// is a nonlegendary hasty Reflection too (and is itself a token, so it can't be chosen again while it lives). Reflection is a word
/// of the Saga's text (CR 612). Nothing happens if the target is no longer legal.
fn copy_as_a_hasty_reflection(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let reflection = g.words(item.object).subtype(Subtype::Reflection);
    let values = g.copiable_values(creature).nonlegendary().with_subtype(reflection).with_keyword(Keyword::Haste);
    g.create_token_copies_of(values, item.controller, 1)?;
    Ok(())
}

/// III: "Sacrifice all Reflections you control." (CR 701.21, 205.3m): each permanent the Saga's controller controls as it
/// resolves that is a Reflection, a creature or anything else, together and with no choice.
fn sacrifice_all_reflections(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let reflection = g.words(item.object).subtype(Subtype::Reflection);
    let reflections: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(reflection)).collect();
    g.sacrifice(item.controller, &reflections);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, copy_as_a_hasty_reflection, "I \u{2014} Choose target nontoken creature you control that doesn't have the same name as a token you control. Create a token that's a copy of it, except it isn't legendary, is a Reflection in addition to its other types, and has haste.")
            .targets(&[target(nontoken_creature_you_control_with_no_token_of_its_name)]),
        chapter(2, copy_as_a_hasty_reflection, "II \u{2014} Choose target nontoken creature you control that doesn't have the same name as a token you control. Create a token that's a copy of it, except it isn't legendary, is a Reflection in addition to its other types, and has haste.")
            .targets(&[target(nontoken_creature_you_control_with_no_token_of_its_name)]),
        chapter(3, sacrifice_all_reflections, "III \u{2014} Sacrifice all Reflections you control."),
    ],
    ..oracle::THE_APPRENTICES_FOLLY
};
