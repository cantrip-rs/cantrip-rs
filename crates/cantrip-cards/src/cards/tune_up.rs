use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Return target artifact card from your graveyard to the battlefield. If it's a Vehicle, it becomes an artifact creature."
/// (CR 400.3, 205.1b, 613.1d): the target, if still legal (CR 608.2b), enters under your control; the next sentence
/// (CR 608.2c) looks at what came onto the battlefield - a card a replacement effect sent elsewhere is not a Vehicle there to
/// change - and makes it a creature as well as an artifact with no duration, so for as long as it stays on the battlefield.
fn return_it_and_vehicle_becomes_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let entered = g.put_onto_battlefield(card, item.controller, false)?;
    g.end_sentence();
    let vehicle = g.words(item.object).subtype(Subtype::Vehicle);
    if g.state().obj(entered).zone.is(ZoneKind::Battlefield) && g.chars(entered).has_subtype(vehicle) {
        instruction(g, |g| {
            g.add_effect(item.object, entered, EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact, CardType::Creature])), Duration::Indefinite);
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::ARTIFACT_CARD_IN_YOUR_GRAVEYARD], return_it_and_vehicle_becomes_a_creature), ..oracle::TUNE_UP };
