use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// A creature declared as a blocker triggers once, even when blocking a band (CR 509.3a).
fn blocks(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Blocks { blocker, declared: true, .. } if *blocker == asker.source => Fired::PLAIN,
        _ => None,
    }
}

/// A lost flip removes the Efreet and unblocks attackers blocked only by it (CR 608.2c, 509.1h).
fn flip(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let won = g.flip_coin(item.controller);
    g.end_sentence();
    if !won {
        let blocker = item.object;
        let attackers: Vec<_> = g.creatures_blocked_by(blocker).into_iter().filter(|&attacker| {
            let mut seen = false;
            for record in history::this_combat(g) {
                if let Event::Blocks { blocker: other, attacker: blocked, .. } = record.event {
                    if blocked == attacker {
                        if other != blocker { return false; }
                        seen = true;
                    }
                }
            }
            seen
        }).collect();
        instruction(g, |g| {
            g.remove_from_combat(blocker);
            g.add_effect(item.object, blocker, EffectKind::CantBlock, Duration::EndOfTurn);
        });
        g.end_sentence();
        instruction(g, |g| {
            for attacker in attackers { g.become_unblocked(attacker); }
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::BLOCKS, blocks),
        flip,
        "Whenever this creature blocks, flip a coin. If you lose the flip, remove this creature from combat and it can't block this turn. Creatures it was blocking that had become blocked by only this creature this combat become unblocked.",
    )],
    ..oracle::YDWEN_EFREET
};
