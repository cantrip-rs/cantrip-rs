use cantrip_api::*;
use cantrip_mechanics::history::permanents_sacrificed_this_turn;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "You lose 1 life and create a Clue token." (CR 119.3, 111.10f.) Creating a Clue is not investigating (the ruling), so
/// nothing that triggers on investigating sees it.
fn lose_life_and_create_a_clue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(item.controller, 1);
    tokens::create(g, item.controller, &tokens::CLUE, 1)?;
    Ok(())
}

/// "Put X +1/+1 counters on target attacking creature, where X is the number of permanents you've sacrificed this turn. If X is
/// three or more, that creature gains lifelink until end of turn." X is worked out as the ability resolves (CR 608.2h). A
/// target that is no longer an attacking creature makes the ability do nothing (CR 608.2b).
fn counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = permanents_sacrificed_this_turn(g, item.controller);
    g.add_counters(creature, CounterKind::PlusOne, x.min(i16::MAX as usize) as i16);
    if x >= 3 {
        effects::gain_lifelink_until_end_of_turn(g, item.object, creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS_OR_YOUR_UPKEEP,
            lose_life_and_create_a_clue,
            "When this enchantment enters and at the beginning of your upkeep, you lose 1 life and create a Clue token.",
        ),
        triggered(
            triggers::YOU_ATTACK,
            counters,
            "Whenever you attack, put X +1/+1 counters on target attacking creature, where X is the number of permanents you've sacrificed this turn. If X is three or more, that creature gains lifelink until end of turn.",
        )
        .targets(&[targets::ATTACKING_CREATURE]),
    ],
    ..oracle::OBSESSIVE_PURSUIT
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
