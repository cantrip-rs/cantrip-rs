use cantrip_api::*;
use cantrip_mechanics::{delayed, exile_target_until_leaves_effect, targets, triggers};
use cantrip_oracle as oracle;

/// "For each opponent, create a tapped token that's a copy of the exiled card attacking that opponent. At the beginning of your
/// next end step, sacrifice those tokens." (CR 707.2, 508.4, 603.7c, 610.3): the copy is of the printed card the Siegebreaker
/// has exiled, nothing it had before (CR 707.2), so an exiled token, which has ceased to exist, is no card to copy, and
/// nothing is copied once the Siegebreaker has left and the card has come back. The tokens were never declared as attackers
/// (CR 508.4), each is created attacking its opponent, all together as the one instruction they are (CR 101.4, 603.2c), and the
/// delayed ability names exactly those tokens (CR 603.7c). A further copy a replacement makes for an opponent attacks that
/// opponent too; a token a replacement adds of its own attacks what its controller chooses (`EntersAttacking`).
fn copy_the_exiled_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = g.exiled_with(g.linked(item)).into_iter().next() else { return Ok(()) };
    let values = g.copiable_values(card);
    let opponents: Vec<EntersAttacking> = g.state().opponents(item.controller).map(|p| EntersAttacking::exactly(Target::Player(p))).collect();
    let tokens = g.create_token_copies_attacking_each(values, item.controller, true, &opponents)?;
    delayed::at_your_next_end_step_about(g, item, 0, &tokens);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            exile_target_until_leaves_effect,
            "When this creature enters, exile up to one other target creature you control until this creature leaves the battlefield.",
        )
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL.optional()]),
        triggered(
            triggers::THIS_ATTACKS,
            copy_the_exiled_card,
            "Whenever this creature attacks, for each opponent, create a tapped token that's a copy of the exiled card attacking that opponent. At the beginning of your next end step, sacrifice those tokens.",
        ),
    ],
    delayed: &[delayed_trigger(delayed::sacrifice_created, "At the beginning of your next end step, sacrifice those tokens.")],
    ..oracle::MARDU_SIEGEBREAKER
};
