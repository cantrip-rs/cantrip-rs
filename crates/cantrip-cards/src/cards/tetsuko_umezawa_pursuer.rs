use cantrip_api::*;
use cantrip_mechanics::{effects::ping_subject_controller, triggers::PROWESS};
use cantrip_oracle as oracle;

/// "Whenever a creature an opponent controls with power or toughness 1 or less blocks" (CR 509.3a/f):
/// one trigger per blocking creature, with its characteristics and opponent control judged at declaration.
fn small_opponent_creature_blocks(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Blocks { blocker, declared: true, .. }
            if q.state().opponents(me.controller).any(|p| p == q.controller(*blocker)) && {
                let c = q.chars(*blocker);
                c.power.is_some_and(|p| p <= 1) || c.toughness.is_some_and(|t| t <= 1)
            } =>
        {
            Fired::about(Target::Object(*blocker))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        PROWESS,
        triggered(
            when_asked(EventMask::BLOCKS, small_opponent_creature_blocks),
            ping_subject_controller,
            "Whenever a creature an opponent controls with power or toughness 1 or less blocks, Tetsuko Umezawa deals 1 damage to that creature's controller.",
        ),
    ],
    ..oracle::TETSUKO_UMEZAWA_PURSUER
};
