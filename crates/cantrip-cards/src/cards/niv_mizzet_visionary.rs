use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever a source you control deals noncombat damage to an opponent" (CR 120.3, 603.2): any source its controller controls as it
/// dealt the damage (a spell, an ability's source or a permanent, even one that has since left; `Event::Damage::controller`), to an
/// opponent as a player and not a permanent they control, and of more than zero (damage prevented to nothing is not dealt). Each
/// source's damage is its own trigger (CR 603.2c), and `damage_amount` keeps that source's amount for "that many".
fn noncombat_damage_you_deal_to_an_opponent(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Player(p), amount, kind, controller, .. }
            if !kind.is_combat() && *amount > 0 && *controller == me.controller && q.state().opponents(me.controller).any(|o| o == *p) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, noncombat_damage_you_deal_to_an_opponent),
        effects::draw_that_many,
        "Whenever a source you control deals noncombat damage to an opponent, you draw that many cards.",
    )
    .damage_amount()],
    play_rules: &[PlayRule::MaximumHandSize(None)],
    ..oracle::NIV_MIZZET_VISIONARY
};
