use cantrip_api::*;
use cantrip_mechanics::{effects, library, select, tokens, triggers};
use cantrip_oracle as oracle;

/// "You may sacrifice another creature. If you do, put a +1/+1 counter on this creature and exile the top card of your library. You
/// may play that card this turn." (CR 608.2c, 701.21): the sacrifice is a choice made as the ability resolves, so a player with no
/// other creature is not asked. If this creature has left the battlefield the counter has nowhere to go (CR 400.7), but the rest
/// is still done (the ruling).
fn sacrifice_another_creature_for_a_counter_and_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::another_creature_permanents(select::up_to(1));
    if effects::may_sacrifice(g, item.controller, item.object, rule)?.objects().is_empty() {
        return Ok(());
    }
    effects::plus_one_counter_on_this(g, item)?;
    library::exile_top_card_play_this_turn(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            tokens::create_rats_that_cant_block::<2>,
            "When this creature enters, create two 1/1 black Rat creature tokens with \"This token can't block.\"",
        ),
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_another_creature_for_a_counter_and_a_card,
            "At the beginning of your upkeep, you may sacrifice another creature. If you do, put a +1/+1 counter on this creature and exile the top card of your library. You may play that card this turn.",
        ),
    ],
    ..oracle::REDCAP_GUTTER_DWELLER
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
