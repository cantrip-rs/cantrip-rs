use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever an opponent loses life" (CR 119.3, 603.2): one event for each time life is lost, damage and paying life
/// included, and none when none is (a loss of 0, or damage that was prevented). "An opponent" is read against the
/// ability's controller (CR 603.7d-e), and a player who has left the game is no one's opponent (CR 800.4a). The trigger
/// carries how much was lost.
fn opponent_loses_life(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::LifeChange { player, delta } if *delta < 0 && q.state().opponents(me.controller).any(|p| p == *player) => {
            Some(Fired { subject: Target::Player(*player), data: [u32::try_from(delta.unsigned_abs()).expect("actual physical life delta fits u32"), 0] })
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when_asked(EventMask::LIFE_CHANGE, opponent_loses_life), cantrip_mechanics::effects::gain_that_much_life, "Whenever an opponent loses life, you gain that much life. (Damage causes loss of life.)")],
    ..oracle::BLOODTHIRSTY_CONQUEROR
};
