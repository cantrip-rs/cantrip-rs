use cantrip_api::*;
use cantrip_mechanics::start_your_engines::has_max_speed;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "It deals 2 damage to each player who doesn't have max speed" (CR 120.3, 702.179e, 702.179f, 120.2): judged as the ability resolves, a
/// player with no speed at all (never had Start your engines!) doesn't have max speed either. Dealt together, by this enchantment
/// as it last existed (CR 113.7a): it was sacrificed to pay for it.
fn two_damage_to_each_player_without_max_speed(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, u32)> =
        g.state().apnap().filter(|&p| !has_max_speed(g.state(), p)).map(|p| (item.object, Target::Player(p), 2)).collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[
        triggered(THIS_ENTERS, effects::damage_first_target::<5>, "When this enchantment enters, it deals 5 damage to up to one target creature or planeswalker.")
            .targets(&[up_to_one(targets::CREATURE_OR_PLANESWALKER.filter)]),
        ability(cost("{2}").and(SACRIFICE_THIS), two_damage_to_each_player_without_max_speed, "{2}, Sacrifice this enchantment: It deals 2 damage to each player who doesn't have max speed."),
    ],
    ..oracle::OUTPACE_OBLIVION
};
