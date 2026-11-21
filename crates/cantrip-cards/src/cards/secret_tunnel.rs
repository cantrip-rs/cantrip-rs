use cantrip_api::*;
use cantrip_mechanics::{select, targets};
use cantrip_oracle as oracle;

/// "Two target creatures you control that share a creature type can't be blocked this turn" (CR 509.1b, 611.2): an effect on each
/// of the two, not an ability they gain (CR 113.1a). If one has left the battlefield the pair is no longer a legal set
/// (CR 608.2b, `TargetSpec::together`, judged with last-known information), and the other is still affected as long as it has a
/// creature type the other had (Secret Tunnel's ruling, 2025-10-02).
fn two_cant_be_blocked_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let pair: Vec<ObjectId> = [0, 1].into_iter().filter_map(|i| targets::object(g, item, i)).collect();
    // One instruction (`Game::one_instruction`): both creatures at once (CR 608.2c).
    instruction(g, |g| {
        for creature in pair {
            g.add_effect(item.object, creature, EffectKind::CantBeBlocked, Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
            cost("{4}, {T}"),
            two_cant_be_blocked_this_turn,
            "{4}, {T}: Two target creatures you control that share a creature type can't be blocked this turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL.between(2, 2).together(select::SHARE_A_CREATURE_TYPE)]),
    ],
    ..oracle::SECRET_TUNNEL
};
