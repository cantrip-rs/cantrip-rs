use cantrip_api::*;
use cantrip_mechanics::{replacements, triggers};
use cantrip_oracle as oracle;

/// "If one or more counters would be put on a permanent you control, that many plus one of each of those kinds of counters
/// are put on that permanent instead" (CR 122.6, 614.1a): the permanent's controller determines whether this replacement
/// applies, and each kind being put on it is increased by one.
fn counters_on_your_permanent(
    q: &dyn Query,
    asker: &Asker,
    _: ObjectId,
    proposal: &Proposal,
) -> bool {
    matches!(proposal, Proposal::Counters(c)
        if c.controller == asker.controller
            && (c.entering || q.is_on_battlefield(c.object))
            && c.counts.iter().any(|&count| count > 0))
}

fn one_more_of_each_kind(_: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    replacements::plus_one_of_each_kind_apply(proposal);
}

/// "Whenever Brad Boimler becomes tapped, until end of turn" (CR 603.2e, 611.2a): each trigger gives its controller a
/// replacement effect about the game, so it continues through Brad leaving the battlefield.
fn add_counters_plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_player_effect(
        item.controller,
        item.object,
        EffectKind::ReplacementRule { card: item.ability.card, index: 0 },
        Duration::EndOfTurn,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_BECOMES_TAPPED,
        add_counters_plus_one,
        "Whenever Brad Boimler becomes tapped, until end of turn, if one or more counters would be put on a permanent you control, that many plus one of each of those kinds of counters are put on that permanent instead.",
    )],
    granted_replacements: &[granted_replacement(
        ReplacementKind::Counters,
        counters_on_your_permanent,
        one_more_of_each_kind,
        "puts that many plus one of each of those kinds of counters instead",
    )],
    ..oracle::BRAD_BOIMLER_EAGER_ENSIGN
};

