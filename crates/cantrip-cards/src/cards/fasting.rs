use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, put a hunger counter on this enchantment. Then destroy this enchantment if it has five
/// or more hunger counters on it." (CR 122.1, 701.8a.) The counter first, then the count read (the ruling): the fifth
/// counter is what destroys it. Not an intervening "if" (CR 603.4): the ability triggers every upkeep. An enchantment that
/// has left the battlefield gets no counter (CR 122.6) and is no longer the object to destroy (CR 400.7).
fn hunger_counter_then_destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Hunger, 1);
    if g.obj(item.object).counters(CounterKind::Hunger) >= 5 {
        g.destroy(item.object);
    }
    Ok(())
}

/// "If you do, you gain 2 life."
fn gain_2_life(g: &mut dyn Game, asker: &Asker) -> Result<(), Suspend> {
    g.gain_life(asker.controller, 2);
    Ok(())
}

const SKIP_DRAW_STEP: Question =
    Question::new("SkipDrawStep", "skip your draw step?", "If you do, you gain 2 life", "Skip it, gain 2 life", "Decline").no_default();

/// "When you draw a card, destroy this enchantment." (CR 701.8a.)
fn destroy_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.destroy(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            hunger_counter_then_destroy,
            "At the beginning of your upkeep, put a hunger counter on this enchantment. Then destroy this enchantment if it has five or more hunger counters on it.",
        ),
        triggered(triggers::YOU_DRAW_A_CARD, destroy_this, "When you draw a card, destroy this enchantment."),
    ],
    // "If you would begin your draw step, you may skip that step instead. If you do, you gain 2 life." (CR 614.10, 500.11.)
    // Chosen as the draw step would begin (the ruling).
    step_begin: &[StepBeginRule { on_skip: gain_2_life, question: &SKIP_DRAW_STEP }],
    ..oracle::FASTING
};
