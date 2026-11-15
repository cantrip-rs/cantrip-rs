use cantrip_api::*;
use cantrip_mechanics::{copy, delayed, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Exile target nonlegendary creature you control. Shards you control become copies of it until the next end step. Return it to
/// the battlefield under its owner's control at the beginning of the next end step." The copiable values are those of the creature
/// as it last existed on the battlefield (CR 707.2, 608.2h): read before it is exiled, so a creature that was itself a copy gives
/// what it copied, and none of its counters, attachments or non-copy effects come with them. The Shards are those you control as
/// the ability resolves (CR 611.2c), and the copy effect ends as the next end step begins, before the return trigger resolves
/// (CR 500.4). Nothing happens if the target is no longer legal (CR 608.2b: the ruling).
fn exile_it_and_shards_become_copies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(creature);
    delayed::exile_target_then_return_at_next_end_step(g, item, 0, 0);
    let shard = g.words(item.object).subtype(Subtype::Shard);
    let shards: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.controller(id) == item.controller && g.chars(id).has_subtype(shard)).collect();
    copy::become_copies(g, item.object, &shards, values, Duration::UntilNextStepOf { step: Step::End, player: None });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            |g, item| tokens::create(g, item.controller, &tokens::SHARD, 2).map(|_| ()),
            "When Niko enters, create two Shard tokens. (They're enchantments with \"{2}, Sacrifice this token: Scry 1, then draw a card.\")",
        ),
        ability(
            cost("{2}").and(TAP),
            exile_it_and_shards_become_copies,
            "{2}, {T}: Exile target nonlegendary creature you control. Shards you control become copies of it until the next end step. Return it to the battlefield under its owner's control at the beginning of the next end step.",
        )
        .targets(&[targets::NONLEGENDARY_CREATURE_YOU_CONTROL]),
    ],
    delayed: &[delayed_trigger(delayed::return_subject_to_battlefield, "Return it to the battlefield under its owner's control at the beginning of the next end step.")],
    ..oracle::NIKO_LIGHT_OF_HOPE
};

pub static EXTRA: &[&CardDef] = &[&tokens::SHARD];
