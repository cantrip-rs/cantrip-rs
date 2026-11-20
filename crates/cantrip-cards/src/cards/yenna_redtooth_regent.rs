use cantrip_api::*;
use cantrip_mechanics::{effects, same_name};
use cantrip_oracle as oracle;

/// "Target enchantment you control that doesn't have the same name as another permanent you control" (CR 115.1, 201.2, 608.2b): an
/// enchantment on the battlefield under the ability's controller whose name no other permanent they control shares, tokens and
/// copies by the names they have now (`same_name`). It is judged again as the ability resolves.
fn enchantment_you_control_with_a_name_of_its_own(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id)
        if q.is_on_battlefield(id)
            && q.controller(id) == asker.controller
            && q.chars(id).is(CardType::Enchantment)
            && !q.permanents_of(asker.controller).into_iter().any(|other| other != id && same_name(q, other, id)))
}

/// "Choose target enchantment you control that doesn't have the same name as another permanent you control. Create a token that's a
/// copy of it, except it isn't legendary. If the token is an Aura, untap Yenna, then scry 2." (CR 707.2, 707.9b, 111.1, 608.2b,
/// 303.4f): the copy exception removes legendary from its copiable values. An Aura token chooses what it enchants as it enters; one
/// that can't enter (nothing for it to enchant) never exists, so is no Aura token to untap Yenna for. Nothing happens if the target
/// is no longer legal.
fn copy_the_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(enchantment) = cantrip_mechanics::targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(enchantment).nonlegendary();
    let made = g.create_token_copies_of(values, item.controller, 1)?;
    g.end_sentence();
    if made.iter().any(|&token| g.chars(token).has_subtype(Subtype::Aura)) {
        effects::untap_self(g, item)?;
        g.end_sentence();
        effects::scry_two(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}, {T}"),
        copy_the_enchantment,
        "{2}, {T}: Choose target enchantment you control that doesn't have the same name as another permanent you control. Create a token that's a copy of it, except it isn't legendary. If the token is an Aura, untap this creature, then scry 2. Activate only as a sorcery.",
    )
    .targets(&[target(enchantment_you_control_with_a_name_of_its_own)])
    .sorcery_speed()],
    ..oracle::YENNA_REDTOOTH_REGENT
};
