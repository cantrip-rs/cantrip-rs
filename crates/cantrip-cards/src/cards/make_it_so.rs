use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Untap target permanent you control. Target player draws two cards": the permanent in slot 0 is untapped if it is still
/// legal (CR 608.2b), and the player in slot 1 draws two if still legal.
fn untap_and_target_player_draws(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(permanent) = targets::object(g, item, 0) {
        g.untap(permanent);
    }
    effects::target_player_draws(g, item, 1, 2);
    Ok(())
}

/// "Create a 1/1 red Officer creature token and draw two cards" (CR 111.1, 121.1).
fn officer_and_draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_by_rule(g, item.controller, &tokens::OFFICER, 1, false)?;
    effects::draw(g, item, 2)
}

static MODAL: ModalDef = modes(&[
    mode(untap_and_target_player_draws, "Untap target permanent you control. Target player draws two cards.")
        .targets(&[targets::PERMANENT_YOU_CONTROL, targets::PLAYER]),
    mode(officer_and_draw_two, "Create a 1/1 red Officer creature token and draw two cards."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::MAKE_IT_SO };
