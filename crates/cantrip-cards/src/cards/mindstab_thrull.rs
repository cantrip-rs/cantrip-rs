use cantrip_api::*;
use cantrip_mechanics::{discard_chosen, effects::ask_may, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "SacrificeToDiscard",
    "sacrifice this creature to make the defending player discard three cards?",
    "",
    "Sacrifice",
    "Don't",
);

/// The choice is made as the ability resolves (CR 608.2d, 701.21a).
fn sacrifice_to_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let defender = g.defending_player(item.object);
    let sacrificed = g.can_sacrifice(item.controller, item.object)
        && ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))?
        && !g.sacrifice(item.controller, &[item.object]).is_empty();
    if let (true, Some(player)) = (sacrificed, defender) {
        g.end_sentence();
        discard_chosen(g, player, item.object, 3)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::UNBLOCKED, triggers::this_attacks_and_isnt_blocked),
            sacrifice_to_discard,
            "Whenever this creature attacks and isn't blocked, you may sacrifice it. If you do, defending player discards three cards.",
        ),
    ],
    ..oracle::MINDSTAB_THRULL
};
