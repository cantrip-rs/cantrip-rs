use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature loses banding and all 'bands with other' abilities until end of turn." (CR 702.22b, 611.2a).
fn remove_banding(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    // Losing banding also removes all bands-with-other abilities (CR 702.22b).
    g.add_effect(item.object, creature, EffectKind::LoseKeywords(Keywords::of(&[Keyword::Banding])), Duration::EndOfTurn);
    Ok(())
}

/// "Activate only during any upkeep step." (CR 602.5).
fn during_upkeep(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() == Step::Upkeep
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::U), "{T}: Add {U}."),
        ability(TAP, remove_banding, "{T}: Target creature loses banding and all \"bands with other\" abilities until end of turn. Activate only during any upkeep step.")
            .targets(&[targets::CREATURE]).condition(during_upkeep),
    ],
    ..oracle::TOLARIA
};
