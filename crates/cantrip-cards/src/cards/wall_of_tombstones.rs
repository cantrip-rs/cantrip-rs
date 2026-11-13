use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, this creature's base toughness becomes equal to 1 plus the number of creature cards in your graveyard." (CR 608.2, 611.2a).
fn creatures_base_toughness_becomes_equal(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let toughness = g.state().zone(Zone::graveyard(item.controller)).iter()
        .filter(|&&id| g.chars(id).is(CardType::Creature))
        .fold(1i32, |count, _| count + 1);
    g.add_effect(item.object, item.object, EffectKind::SetToughness(toughness), Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            creatures_base_toughness_becomes_equal,
            "At the beginning of your upkeep, this creature's base toughness becomes equal to 1 plus the number of creature cards in your graveyard.",
        ),
    ],
    ..oracle::WALL_OF_TOMBSTONES
};
