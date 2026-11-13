use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::targets;

/// "{T}: Flip a coin. If you win the flip, draw a card. If you lose the flip, counter target artifact spell you control that isn't the target of an ability from another creature named Goblin Artisans." (CR 115.1).
fn flip_coin_if_win_flip_target_0(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::SPELL.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Artifact)
            && asker.controller == q.controller(id)
            && !q.state().stack.iter().any(|item| {
                item.kind != StackKind::Spell
                    && item.object != asker.source
                    && q.state().targets(item).contains(&t)
                    && q.chars(item.object).is(CardType::Creature)
                    && q.chars(item.object).name == "Goblin Artisans"
            }))
}

/// "{T}: Flip a coin. If you win the flip, draw a card. If you lose the flip, counter target artifact spell you control that isn't the target of an ability from another creature named Goblin Artisans." (CR 608.2, 121.1, 701.6a).
fn flip_coin_if_win_flip(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let flip_0 = g.flip_coin(item.controller);
    if flip_0 {
        g.draw(item.controller, 1);
        g.end_sentence();
    } else {
        let objects_2 = t_0
            .get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .into_iter()
            .collect::<Vec<_>>();
        instruction(
            g,
            |g| {
                for spell in objects_2 {
                    g.counter(spell);
                }
            },
        );
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                flip_coin_if_win_flip,
                "{T}: Flip a coin. If you win the flip, draw a card. If you lose the flip, counter target artifact spell you control that isn't the target of an ability from another creature named Goblin Artisans.",
            )
            .targets(&[target(flip_coin_if_win_flip_target_0)]),
    ],
    ..oracle::GOBLIN_ARTISANS
};
