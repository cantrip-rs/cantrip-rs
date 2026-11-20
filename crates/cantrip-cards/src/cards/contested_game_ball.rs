use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Whenever you're dealt combat damage" (CR 120.2, 510.2, 603.2c): combat damage dealt to the ability's controller (a source of power 0 deals none, CR 120.8), the
/// subject being the attacking player, the one who controlled the source of the damage as it dealt it. Damage dealt by several
/// creatures at once is one trigger (`sums_per_subject`).
fn you_are_dealt_combat_damage(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Player(p), kind, controller, .. } if kind.is_combat() && *p == asker.controller => {
            Fired::about(Target::Player(*controller))
        }
        _ => None,
    }
}

/// "The attacking player gains control of this artifact and untaps it" (CR 613.1b, 701.26b): the control change has no end. The untap is
/// done whether or not they took control, and both do nothing for a Ball that has left the battlefield (CR 400.7).
fn the_attacking_player_takes_it_and_untaps_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(attacker) = item.subject.player() {
        g.gain_control(item.object, item.object, attacker, Duration::Indefinite);
        g.untap(item.object);
    }
    Ok(())
}

/// "Draw a card and put a point counter on this artifact. Then if it has five or more point counters on it, sacrifice it and create
/// a Treasure token" (CR 121.1, 122.6, 608.2c, 608.2h): the draw does not depend on the Ball being there, and a counter goes on it
/// only while it is (CR 400.7). The count is read as the first sentence leaves it, and of a Ball that has left as it last was: one
/// that left with five already on it is not sacrificed but still makes the Treasure, the ability controller's (as Treasure Map's
/// ruling has it for its three).
fn draw_and_count_a_point(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    g.add_counters(item.object, CounterKind::Point, 1);
    g.end_sentence();
    if g.obj(item.object).counters(CounterKind::Point) >= 5 {
        g.sacrifice(item.controller, &[item.object]);
        tokens::create_treasure(g, item.controller)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::DAMAGE, you_are_dealt_combat_damage),
            the_attacking_player_takes_it_and_untaps_it,
            "Whenever you're dealt combat damage, the attacking player gains control of this artifact and untaps it.",
        )
        .sums_per_subject(),
        ability(
            cost("{2}, {T}"),
            draw_and_count_a_point,
            "{2}, {T}: Draw a card and put a point counter on this artifact. Then if it has five or more point counters on it, sacrifice it and create a Treasure token.",
        ),
    ],
    ..oracle::CONTESTED_GAME_BALL
};
