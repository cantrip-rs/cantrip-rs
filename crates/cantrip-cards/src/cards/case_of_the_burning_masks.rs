use cantrip_api::*;
use cantrip_mechanics::{effects, library, solved, targets, triggers};
use cantrip_oracle as oracle;

/// "To solve \u{2014} Three or more sources you controlled dealt damage this turn" (CR 719.3a): the distinct sources (CR 400.7: a
/// permanent that left and came back is another) that dealt damage this turn, combat or not, to anything, under the Case's
/// controller's control as they dealt it (the ruling: a source that has since died or changed control still counts), and
/// one source counts once however often it dealt damage. Damage that was prevented to nothing was not dealt (CR 615.6). An
/// intervening "if" (CR 603.4), asked as the end step begins and as the ability resolves.
fn three_sources_you_controlled_dealt_damage(q: &dyn Query, asker: &Asker) -> bool {
    let mut sources: Vec<ObjectId> = q
        .history()
        .select(EventQuery::kind(EventMask::DAMAGE).this_turn())
        .filter_map(|r| match r.event {
            Event::Damage { source, controller, .. } if controller == asker.controller => Some(source),
            _ => None,
        })
        .collect();
    sources.sort();
    sources.dedup();
    sources.len() >= 3
}

/// "Exile the top three cards of your library. Choose one of them. You may play that card this turn" (CR 406, 305.1, 601.3).
fn exile_top_three_play_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let grant = PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST };
    library::exile_top_choose_one_may_play(g, item.controller, item.object, 3, grant)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::damage_first_target::<3>, "When this Case enters, it deals 3 damage to target creature an opponent controls.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        solved::to_solve(
            three_sources_you_controlled_dealt_damage,
            "To solve \u{2014} Three or more sources you controlled dealt damage this turn. (If unsolved, solve at the beginning of your end step.)",
        ),
        solved::ability(SACRIFICE_THIS, exile_top_three_play_one, "Solved \u{2014} Sacrifice this Case: Exile the top three cards of your library. Choose one of them. You may play that card this turn."),
    ],
    ..oracle::CASE_OF_THE_BURNING_MASKS
};
