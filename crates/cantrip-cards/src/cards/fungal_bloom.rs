use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Target Fungus (CR 115.1, 612.2).
fn fungus(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Fungus])
}

/// Put a spore counter on the target (CR 122.1, 608.2b).
fn put_spore(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(target, CounterKind::Spore, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{G}{G}"), put_spore, "{G}{G}: Put a spore counter on target Fungus.")
        .targets(&[target(fungus)])],
    ..oracle::FUNGAL_BLOOM
};
