use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever one or more opponents are dealt noncombat damage" (CR 120.2, 603.2, 603.2c): damage dealt to an opponent of this
/// ability's controller that is not combat damage, and of more than zero (damage prevented or reduced to nothing is not
/// "dealt"). Damage dealt to several opponents at once is one batch, and the ability, `once_per_batch`, triggers once for it.
fn opponent_dealt_noncombat_damage(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Player(p), amount, kind, .. }
            if !kind.is_combat() && *amount > 0 && q.state().opponents(me.controller).any(|o| o == *p) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, opponent_dealt_noncombat_damage),
        effects::creatures_you_control_get::<1, 0>,
        "Whenever one or more opponents are dealt noncombat damage, creatures you control get +1/+0 until end of turn.",
    )
    .once_per_batch()],
    ..oracle::MASTER_OF_BARBS
};
