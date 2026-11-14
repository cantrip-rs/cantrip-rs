use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

static SACRIFICE: Question = Question::new(
    "SacrificeNecrite", "sacrifice this creature?",
    "If you do, destroy the chosen creature; it can't be regenerated", "Sacrifice", "Don't sacrifice",
);

/// An unblocked Necrite may be sacrificed as an effect, not a cost (CR 608.2b-d, 701.21a, 701.8a, 701.19c).
fn sacrifice_and_destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // Targets are checked as resolution begins, before the sacrifice.
    let victim = targets::object(g, item, 0);
    if g.can_sacrifice(item.controller, item.object)
        && effects::ask_may(g, item.controller, item.object, Purpose::Ask(&SACRIFICE))?
        && !g.sacrifice(item.controller, &[item.object]).is_empty()
    {
        g.end_sentence();
        if let Some(victim) = victim {
            g.destroy_no_regen(victim);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::UNBLOCKED, triggers::this_attacks_and_isnt_blocked),
            sacrifice_and_destroy,
            "Whenever this creature attacks and isn't blocked, you may sacrifice it. If you do, destroy target creature defending player controls. It can't be regenerated.",
        ).targets(&[targets::CREATURE_DEFENDING_PLAYER_CONTROLS]),
    ],
    ..oracle::NECRITE
};
