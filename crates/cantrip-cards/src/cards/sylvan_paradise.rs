use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Each legal target becomes only the spell's green until end of turn (CR 608.2b, 612.2, 611.2a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    let colors = g.words(item.object).colors(ColorSet::of(Color::Green));
    instruction(g, |g| {
        for &creature in &creatures {
            g.add_effect(item.object, creature, EffectKind::SetColors(colors), Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.between(1, TargetSpec::ANY_NUMBER)], effect),
    ..oracle::SYLVAN_PARADISE
};
