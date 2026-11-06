use cantrip_api::*;
use cantrip_mechanics::{animate, targets::filters, Animation};
use cantrip_oracle as oracle;

/// Forests controlled when the ability resolves become 2/3 creatures, retaining their types (CR 611.2c).
fn animate_forests(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let forest = g.words(item.object).subtype(Subtype::Forest);
    let forests: Vec<_> = g.permanents_of(item.controller).into_iter()
        .filter(|&id| g.chars(id).has_subtype(forest)).collect();
    let animation = Animation {
        power: 2,
        toughness: 3,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::NONE,
        ability: None,
    };
    instruction(g, |g| {
        for forest in forests {
            animate(g, item.object, forest, &animation, Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{G}").and(TAP).and(sacrifice(1, filters::creature)),
        animate_forests,
        "{1}{G}, {T}, Sacrifice a creature: Forests you control become 2/3 creatures until end of turn. They're still lands.",
    )],
    ..oracle::THELONITE_DRUID
};
