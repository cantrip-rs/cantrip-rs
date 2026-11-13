use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature or Vehicle. If the sacrificed permanent was a Vehicle, draw a card." (CR 701.8, 608.2h): what the
/// additional cost sacrificed, as it last existed on the battlefield. The card is drawn whether or not the destroy did anything.
fn destroy_then_draw_if_a_vehicle_was_sacrificed(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::destroy_first_target(g, item)?;
    g.end_sentence();
    let vehicle = g.history().spell_cost_objects(item.object, CostOrigin::PrintedAdditional, 0, CostPart::Permanents).next().is_some_and(|sacrificed| g.chars(sacrificed).has_subtype(Subtype::Vehicle));
    if vehicle {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_VEHICLE], destroy_then_draw_if_a_vehicle_was_sacrificed),
    // As an additional cost to cast this spell, sacrifice an artifact or creature (CR 118.8, 601.2b).
    additional_cost: Some(Cost::sacrifice(1, targets::filters::artifact_or_creature)),
    ..oracle::HELLISH_SIDESWIPE
};
