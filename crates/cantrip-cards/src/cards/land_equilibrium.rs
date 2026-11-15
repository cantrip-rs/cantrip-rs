use cantrip_api::*;
use cantrip_mechanics::{lands_you_control, sacrifice_chosen};
use cantrip_oracle as oracle;

/// "If an opponent who controls at least as many lands as you do would put a land onto the battlefield" (CR 614.1a): the
/// player putting it there (CR 110.2a), whoever it enters under (the rulings: under another's control, as Yavimaya Dryad
/// puts it, all the same), and however it gets there - played or put by a spell or ability (the rulings). The object is
/// judged as it would exist on the battlefield (CR 614.12), and the lands counted as they are before it arrives. A move has
/// a player putting it only while it is going to the battlefield (`ZoneChange::put_by`).
fn opponent_with_as_many_lands_would_put_a_land(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::ZoneChange(z) = proposal else { return false };
    let Some(player) = z.put_by else { return false };
    q.state().opponents(asker.controller).any(|o| o == player)
        && q.chars(z.object).is(CardType::Land)
        && lands_you_control(q, player) >= lands_you_control(q, asker.controller)
}

/// "... that player instead puts that land onto the battlefield": the land goes onto the battlefield as the event and the
/// other effects have it (CR 614.6, 616.1: its own "enters tapped" too, the rulings).
fn puts_that_land_onto_the_battlefield(_: &dyn Query, _: &Asker, _: &mut Proposal) {}

/// "... then sacrifices a land of their choice": once the land is on the battlefield (CR 614.6), the player who put it there
/// sacrifices a land they control (CR 701.21a), the one that just entered among those they may choose, before anyone could
/// tap it for mana (the rulings) or anything triggered goes on the stack.
fn then_sacrifices_a_land(g: &mut dyn Game, asker: &Asker, _: &Proposal, after: &Proposal) {
    let Proposal::ZoneChange(z) = after else { return };
    let Some(player) = z.put_by else { return };
    let _ = sacrifice_chosen(g, player, asker.source, 1, |c| c.is(CardType::Land));
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        opponent_with_as_many_lands_would_put_a_land,
        puts_that_land_onto_the_battlefield,
        "puts it onto the battlefield, then sacrifices a land",
    )
    .afterward(then_sacrifices_a_land)],
    ..oracle::LAND_EQUILIBRIUM
};
