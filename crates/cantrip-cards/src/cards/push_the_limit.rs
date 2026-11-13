use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, is_mount_or_vehicle};
use cantrip_oracle as oracle;

/// "Return all Mount and Vehicle cards from your graveyard to the battlefield. Sacrifice them at the beginning of the next end step.
/// Vehicles you control become artifact creatures until end of turn. Creatures you control gain haste until end of turn." (CR 603.7c,
/// 611.2c): the cards enter together, and the delayed ability is about exactly the ones that did. A Vehicle that entered is
/// "a Vehicle you control" for the third sentence, and a creature (Mount, or Vehicle made one) is one for the fourth, whichever
/// way it came to be: each set is taken as it resolves, after the sentence before it has been done.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let vehicle = g.words(item.object).subtype(Subtype::Vehicle);
    let cards: Vec<(ObjectId, PlayerId)> = g
        .state()
        .zone(Zone::graveyard(item.controller))
        .iter()
        .copied()
        .filter(|&card| is_mount_or_vehicle(g, item.object, card))
        .map(|card| (card, item.controller))
        .collect();
    let returned = g.put_all_onto_battlefield(&cards)?;
    if !returned.is_empty() {
        delayed::at_next_end_step_about(g, item, 0, &returned);
    }
    let vehicles: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(vehicle)).collect();
    instruction(g, |g| {
        for id in vehicles {
            g.add_effect(item.object, id, EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact, CardType::Creature])), Duration::EndOfTurn);
        }
    });
    g.end_sentence();
    effects::creatures_you_control_gain(g, item, &[Keyword::Haste]);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), delayed: &[delayed::SACRIFICE_CREATED], ..oracle::PUSH_THE_LIMIT };
