use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// "Collision Course deals X damage to target creature, where X is the number of permanents you control that are creatures
/// and/or Vehicles." Counted as the spell resolves, each permanent once even if a crewed Vehicle is both (CR 301.7, 120.3).
fn damage_per_creature_and_vehicle(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let vehicle = g.words(item.object).subtype(Subtype::Vehicle);
    let x = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature) || g.chars(id).has_subtype(vehicle))
        .count();
    damage_target(g, item, 0, x as u32);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(
        damage_per_creature_and_vehicle,
        "Collision Course deals X damage to target creature, where X is the number of permanents you control that are creatures and/or Vehicles.",
    )
    .targets(&[targets::CREATURE]),
    mode(effects::destroy_first_target, "Destroy target artifact.").targets(&[targets::ARTIFACT]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::COLLISION_COURSE };
