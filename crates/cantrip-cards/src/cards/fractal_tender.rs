use cantrip_api::*;
use cantrip_mechanics::{history, increment, tokens, triggers};
use cantrip_oracle as oracle;

/// The intervening "if you put a counter on this creature this turn" (CR 603.4): a counter of any kind, put on it by its
/// controller, asked of the history (CR 122.6) as the end step begins and again as the ability resolves. Counters an
/// opponent put on it do not count; counters it entered with do, for the engine announces them as put on it by its controller.
fn you_put_a_counter_on_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    history::put_a_counter_on_this_turn(q.history(), asker.controller, asker.source)
}

/// "Create a 0/0 green and blue Fractal creature token and put three +1/+1 counters on it." (CR 111.1, 122.1)
fn fractal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_with_counters(g, item.controller, &tokens::FRACTAL, 3)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        increment::TRIGGER,
        triggered(
            triggers::EACH_END_STEP,
            fractal,
            "At the beginning of each end step, if you put a counter on this creature this turn, create a 0/0 green and blue Fractal creature token and put three +1/+1 counters on it.",
        )
        .condition(you_put_a_counter_on_this_turn),
    ],
    ..oracle::FRACTAL_TENDER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FRACTAL];
