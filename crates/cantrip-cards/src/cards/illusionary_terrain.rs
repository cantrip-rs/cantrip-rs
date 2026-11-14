use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, cumulative_upkeep::Payment, text_change};
use cantrip_oracle as oracle;

/// "As this enchantment enters, choose two basic land types." (CR 614.12a).
fn choose_types(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let (first, second) = text_change::two_basic_land_types(g, entering.controller, entering.object, WordUse::FirstChosen, WordUse::SecondChosen)?;
    entering.chosen_subtype_pair = Some([first, second]);
    Ok(())
}

/// "Basic lands of the first chosen type are the second chosen type." (CR 305.7, 607.2d).
fn affects(q: &dyn LayerQuery, source: ObjectId, _: ObjectId, chars: &Chars) -> bool {
    chars.supertypes.basic && chars.is(CardType::Land)
        && q.state().chosen_subtype_pair(q.linked_to(source)).is_some_and(|[first, _]| chars.has_subtype(first))
}

fn change_type(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    if let Some([_, second]) = q.state().chosen_subtype_pair(q.linked_to(source)) {
        chars.set_land_subtypes(SubtypeSet::NONE.with(second));
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[cumulative_upkeep!(Payment::Mana(mana("{2}")), "Cumulative upkeep {2}")],
    statics: &[sets_land_subtypes(affects, change_type, "Basic lands of the first chosen type are the second chosen type.")],
    enters: Some(choose_types),
    ..oracle::ILLUSIONARY_TERRAIN
};
