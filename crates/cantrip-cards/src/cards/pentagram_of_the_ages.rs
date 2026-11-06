use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// The source is chosen as the ability resolves (CR 609.7a, 615.8).
fn prevent_from_source(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::next_time_chosen_source_would_damage_you(g, item, 0)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{4}").and(TAP), prevent_from_source,
        "{4}, {T}: The next time a source of your choice would deal damage to you this turn, prevent that damage.")],
    granted_preventions: &[GrantedPreventionDef {
        text: "The next time a source of your choice would deal damage to you this turn, prevent that damage.",
        ..prevention::FROM_CHOSEN_SOURCE
    }],
    ..oracle::PENTAGRAM_OF_THE_AGES
};
