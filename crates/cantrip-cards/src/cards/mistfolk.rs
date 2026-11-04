use cantrip_api::*;
use cantrip_mechanics::counter_target;
use cantrip_oracle as oracle;

/// "{U}: Counter target spell that targets this creature." (CR 115.1).
fn spell_targeting_this(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.state().stack.iter()
        .find(|i| i.kind == StackKind::Spell && i.object == id)
        .is_some_and(|i| q.state().targets(i).contains(&Target::Object(asker.source))))
}

/// "{U}: Counter target spell that targets this creature." (CR 608.2b, 701.6a).
fn counter_spell_targeting_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{U}"),
        counter_spell_targeting_this,
        "{U}: Counter target spell that targets this creature.",
    ).targets(&[target(spell_targeting_this)])],
    ..oracle::MISTFOLK
};
