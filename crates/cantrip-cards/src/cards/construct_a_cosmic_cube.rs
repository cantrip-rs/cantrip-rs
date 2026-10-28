use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 2/1 black Villain creature token with menace and put a plan counter on this enchantment." (CR 111.1, 122.1): the
/// counter goes only on the enchantment that triggered, if it is still there (CR 400.7); the token is made either way.
fn villain_and_plan(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::VILLAIN_2_1_BLACK_MENACE, 1)?;
    g.end_sentence();
    g.add_counters(item.object, CounterKind::Plan, 1);
    Ok(())
}

/// "You control target opponent during their next turn." (CR 723.1): you make every choice and decision that player would make
/// during the next turn they actually take, seeing what they see; a target that is no longer legal does nothing (CR 608.2b).
fn control_next_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Some(player) = item.target(0).player()
    {
        g.control_next_turn(item, player);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_DRAW_YOUR_SECOND_CARD,
            villain_and_plan,
            "Whenever you draw your second card each turn, create a 2/1 black Villain creature token with menace and put a plan counter on this enchantment.",
        ),
        triggered(
            when_asked(EventMask::COUNTERS, triggers::nth_counter_put_on_this::<{ CounterKind::Plan as u8 }, 7>),
            effects::sacrifice_this_when_you_do,
            "When the seventh plan counter is put on this enchantment, sacrifice it. When you do, you control target opponent during their next turn.",
        ),
    ],
    delayed: &[delayed_trigger(control_next_turn, "When you do, you control target opponent during their next turn.").targets(&[targets::OPPONENT])],
    ..oracle::CONSTRUCT_A_COSMIC_CUBE
};

pub static EXTRA: &[&CardDef] = &[&tokens::VILLAIN_2_1_BLACK_MENACE];
