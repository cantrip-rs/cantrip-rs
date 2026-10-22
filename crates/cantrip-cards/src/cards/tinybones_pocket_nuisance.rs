use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Whenever a player discards one or more cards" (CR 603.2c): group this discard batch separately for each player.
/// EventMask::DISCARDED guarantees the event names the discarding player; the subject is not a target.
fn player_discards(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    let player = event.player().expect("a discard names its player");
    Some(Fired { subject: Target::Player(player), data: [0, player.0 as u32] })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::each_opponent_discards_a_card, "When Tinybones enters, each opponent discards a card."),
        triggered(
            when(EventMask::DISCARDED, player_discards),
            |g, item| {
                effects::damage_each_opponent(g, item, 1);
                Ok(())
            },
            "Whenever a player discards one or more cards, Tinybones deals 1 damage to each opponent.",
        )
        .once_per_batch()
        .batch_by_data(),
    ],
    ..oracle::TINYBONES_POCKET_NUISANCE
};
