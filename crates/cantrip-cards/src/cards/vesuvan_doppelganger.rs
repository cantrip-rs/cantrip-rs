use cantrip_api::*;
use cantrip_mechanics::{copy, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Except it doesn't copy that creature's color and it has [this ability]": color is left out of the copying, so it is what
/// the values beneath give (CR 707.9c, 613.2a), and the copied values gain `ability` (CR 707.9a).
fn copy_values(q: &dyn Query, original: ObjectId, ability: AbilityRef) -> Copiable {
    q.copiable_values(original).uncopied_color().with_ability(ability)
}

/// "At the beginning of your upkeep, you may have this creature become a copy of target creature, except it doesn't copy that creature's color and it has this ability." (CR 608.2).
fn granted_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    const ASK: Question = Question::new("Copy", "become a copy of the targeted creature?", "It keeps its color and this ability", "Become a copy", "Keep current form");
    let Some(original) = g.state().target(item, 0).object() else { return Ok(()) };
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        let values = copy_values(g, original, item.ability);
        instruction(g, |g| g.become_copy(item.object, item.object, values, Duration::Indefinite));
        g.end_sentence();
    }
    Ok(())
}

/// "You may have this creature enter as a copy of any creature on the battlefield, except it doesn't copy that creature's color and it has "At the beginning of your upkeep, you may have this creature become a copy of target creature, except it doesn't copy that creature's color and it has this ability."" (CR 614.1c).
/// The ability it has is this card's, whatever object applies the replacement: a Clone copying a Doppelganger that copies
/// nothing applies it too (CR 707.9e, 614.12).
fn have_creature_enter_as_copy_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    let candidates: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| id != entering.object && g.chars(id).is(CardType::Creature))
        .collect();
    let Some(chosen) = copy::choose_copy_of_one_of(g, entering, &candidates, &copy::COPY_AS_ENTERS)? else { return Ok(()) };
    let ability = AbilityRef::delayed(g.card_id(&CARD), 0);
    entering.copy = Some(copy_values(g, chosen, ability));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    delayed: &[
        triggered(
                triggers::YOUR_UPKEEP,
                granted_1,
                "At the beginning of your upkeep, you may have this creature become a copy of target creature, except it doesn't copy that creature's color and it has this ability.",
            )
            .targets(&[targets::CREATURE]),
    ],
    enters_as_copy: Some(have_creature_enter_as_copy_enters),
    ..oracle::VESUVAN_DOPPELGANGER
};
