use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, create a Food token." (CR 111.10b)
fn create_a_food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_food(g, item.controller)
}

/// "Whenever you gain life for the first time during each of your turns, create a 1/1 white Cat creature token" (CR 603.2): the
/// first life gain of the turn counts, and only on its controller's own turn.
fn gained_life_first_time_on_your_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if q.state().turn.active == me.controller {
        triggers::you_gain_life_for_the_first_time_each_turn(q, me, event)
    } else {
        None
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, create_a_food, "When this creature enters, create a Food token."),
        triggered(
            when_asked(EventMask::LIFE_CHANGE, gained_life_first_time_on_your_turn),
            tokens::create_a_cat,
            "Whenever you gain life for the first time during each of your turns, create a 1/1 white Cat creature token.",
        ),
    ],
    ..oracle::CAT_COLLECTOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::CAT, &tokens::FOOD];
