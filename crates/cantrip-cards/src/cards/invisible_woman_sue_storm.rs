use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "CreateToken",
    "create a 0/4 Wall token?",
    "A 0/4 colorless Wall creature token with defender",
    "Create a Wall",
    "Don't",
);

/// "Whenever you put one or more +1/+1 counters on one or more other Heroes you control" (CR 122.6a, 603.2c): the counters are put
/// by this ability's controller (the event's `by`, CR 122.6a: an opponent's effect putting them on your Hero does not trigger it),
/// on a Hero its controller controls other than this creature - the type read through the words of this card's text (CR 612) -
/// including one that enters with them. Counters put at the same time on several Heroes are one batch, so it triggers once
/// (`once_per_batch`).
fn you_put_plus_one_counters_on_another_hero_you_control(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, kind: CounterKind::PlusOne, amount, by, .. }
            if *amount > 0
                && *by == me.controller
                && *object != me.source
                && q.controller(*object) == me.controller
                && q.chars(*object).has_subtype(q.words(me.source).subtype(Subtype::Hero)) =>
        {
            Fired::about(Target::Object(*object))
        }
        _ => None,
    }
}

/// "You may create a 0/4 colorless Wall creature token with defender." (CR 111.2, 608.2d): its controller chooses as it resolves.
fn may_create_a_wall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        tokens::create(g, item.controller, &tokens::WALL_0_4_DEFENDER, 1)?;
    }
    Ok(())
}

/// Lifelink is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::COUNTERS, you_put_plus_one_counters_on_another_hero_you_control),
        may_create_a_wall,
        "Whenever you put one or more +1/+1 counters on one or more other Heroes you control, you may create a 0/4 colorless Wall creature token with defender.",
    )
    .once_per_batch()],
    ..oracle::INVISIBLE_WOMAN_SUE_STORM
};

pub static EXTRA: &[&CardDef] = &[&tokens::WALL_0_4_DEFENDER];
