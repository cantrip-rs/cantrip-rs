use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever a player attacks with `N` or more creatures" (CR 508.1m, 603.2): any player's declaration of attackers, the
/// Angel's controller's included, judged on the creatures declared as attackers. It does not matter what happens to them
/// afterwards (the ruling).
fn a_player_attacks_with_at_least<const N: u32>(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::AttackersDeclared { count, .. } if *count >= N => Fired::PLAIN,
        _ => None,
    }
}

/// "Aurelia deals 3 damage to each of your opponents and you gain 3 life": the life is gained whether or not the damage was
/// dealt (CR 608.2c).
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent_and_gain_life(g, item, 3)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when(EventMask::ATTACKERS_DECLARED, a_player_attacks_with_at_least::<3>),
            |g, item| effects::draw(g, item, 1),
            "Whenever a player attacks with three or more creatures, you draw a card.",
        ),
        triggered(
            when(EventMask::ATTACKERS_DECLARED, a_player_attacks_with_at_least::<5>),
            drain,
            "Whenever a player attacks with five or more creatures, Aurelia deals 3 damage to each of your opponents and you gain 3 life.",
        ),
    ],
    ..oracle::AURELIA_THE_LAW_ABOVE
};
