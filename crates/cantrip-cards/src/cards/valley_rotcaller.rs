use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Each opponent loses X life and you gain X life, where X is the number of other Squirrels, Bats, Lizards, and Rats you
/// control" (CR 119.3, 205.3, 608.2h): X is counted as the ability resolves over the permanents its controller controls but
/// its source, each read through the words of the source's text (CR 612.2); a Squirrel Bat counts once.
fn each_opponent_loses_x_life_and_you_gain_x_life_for_other_squirrels_bats_lizards_and_rats(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let words = [Subtype::Squirrel, Subtype::Bat, Subtype::Lizard, Subtype::Rat].map(|subtype| g.words(item.object).subtype(subtype));
    let x = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| id != item.object && words.iter().any(|&word| g.chars(id).has_subtype(word)))
        .count();
    let x = i32::try_from(x).unwrap_or(i32::MAX);
    g.each_opponent_loses_life(item.controller, x);
    g.gain_life(item.controller, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        each_opponent_loses_x_life_and_you_gain_x_life_for_other_squirrels_bats_lizards_and_rats,
        "Whenever this creature attacks, each opponent loses X life and you gain X life, where X is the number of other Squirrels, Bats, Lizards, and Rats you control.",
    )],
    ..oracle::VALLEY_ROTCALLER
};
