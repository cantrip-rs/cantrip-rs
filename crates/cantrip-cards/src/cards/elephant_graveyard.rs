use cantrip_api::*;
use cantrip_mechanics::{regenerate, targets};
use cantrip_oracle as oracle;

/// Target an Elephant as the land's text names it (CR 115.1, 612.2).
fn elephant(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Elephant])
}

fn regenerate_elephant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    regenerate::target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(TAP, regenerate_elephant, "{T}: Regenerate target Elephant.").targets(&[target(elephant)]),
    ],
    ..oracle::ELEPHANT_GRAVEYARD
};
