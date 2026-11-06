use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, effects, targets};
use cantrip_oracle as oracle;

/// "Sacrifice this artifact: Flip a coin. If you win the flip, this artifact deals damage to target opponent or planeswalker equal to the number of creatures you control. If you lose the flip, this artifact deals damage to you equal to the number of creatures that opponent or that planeswalker's controller controls." (CR 115.1).
fn flip_coin_if_win_flip_target_0(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    match t {
        Target::Player(p) => {
            q.state().opponents(asker.controller).any(|opponent| opponent == p)
        }
        Target::Object(id) => {
            targets::permanent(q, id) && q.chars(id).is(CardType::Planeswalker)
        }
        _ => false,
    }
}

/// "Sacrifice this artifact: Flip a coin. If you win the flip, this artifact deals damage to target opponent or planeswalker equal to the number of creatures you control. If you lose the flip, this artifact deals damage to you equal to the number of creatures that opponent or that planeswalker's controller controls." (CR 608.2, 120.3).
fn flip_coin_if_win_flip(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let flip_0 = g.flip_coin(item.controller);
    g.end_sentence();
    if flip_0 {
        effects::damage_first_target_by_creatures_you_control(g, item)?;
        g.end_sentence();
    } else {
        let player = t_0[0].player().or_else(|| t_0[0].object().map(|object| g.controller(object)));
        let amount = player.map_or(0, |player| creatures_you_control(g, player) as u32);
        g.damage(item.object, Target::Player(item.controller), amount);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                SACRIFICE_THIS,
                flip_coin_if_win_flip,
                "Sacrifice this artifact: Flip a coin. If you win the flip, this artifact deals damage to target opponent or planeswalker equal to the number of creatures you control. If you lose the flip, this artifact deals damage to you equal to the number of creatures that opponent or that planeswalker's controller controls.",
            )
            .targets(&[target(flip_coin_if_win_flip_target_0)]),
    ],
    ..oracle::GOBLIN_LYRE
};
