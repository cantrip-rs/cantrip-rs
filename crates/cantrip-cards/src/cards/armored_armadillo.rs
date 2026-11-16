use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "This creature gets +X/+0 until end of turn, where X is its toughness" (CR 611.2a, 608.2h): X is its toughness as the ability
/// resolves, as the layers make it (a counter or an earlier pump counts), and is not recalculated later in the turn (the ruling).
/// A creature that has left the battlefield gets nothing (CR 400.7). X is never negative: state-based actions ran before the
/// last priority, so a creature still on the battlefield has toughness 1 or more (CR 704.5f); a noncreature permanent has none.
fn gets_plus_its_toughness_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.is_on_battlefield(item.object) {
        let x = g.chars(item.object).toughness.unwrap_or_default();
        cantrip_mechanics::pump(g, item.object, item.object, x, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)"),
        ability(cost("{3}{W}"), gets_plus_its_toughness_power, "{3}{W}: This creature gets +X/+0 until end of turn, where X is its toughness."),
    ],
    ..oracle::ARMORED_ARMADILLO
};
