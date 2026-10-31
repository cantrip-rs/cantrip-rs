use cantrip_api::*;
use cantrip_mechanics::replacements;
use cantrip_oracle as oracle;

/// "If you would put one or more counters on a permanent you control" (CR 122.6, 614.1a): the counters of any kinds that its
/// controller puts - the player who puts them is the event's, so an opponent's effect putting counters on your permanent is not
/// replaced - on a permanent that player controls, on the battlefield or entering under them with counters (CR 614.12), whether an
/// effect or a cost puts them.
fn you_put_counters_on_your_permanent(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::Counters(c) = proposal else { return false };
    c.putter == asker.controller && c.controller == asker.controller && (c.entering || q.is_on_battlefield(c.object)) && c.counts.iter().any(|&n| n > 0)
}

/// "... put that many plus one of each of those kinds of counters on that permanent instead" (CR 122.6, 614.1a).
fn that_many_plus_one(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    replacements::plus_one_of_each_kind_apply(proposal);
}

/// "{T}: Add X mana of any one color, where X is Doc Samson's power." (CR 106.5, 605.1a): his power as the ability resolves, the
/// layers' answer (CR 613); with none or less than none, no mana is added.
fn add_power_mana_of_one_color(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    let power = g.chars(asker.source).power.unwrap_or(0);
    if let Some(color) = color
        && power > 0
    {
        g.add_mana(asker.controller, color.mana(), power as u32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_by(add_power_mana_of_one_color).choose_color(), "{T}: Add X mana of any one color, where X is Doc Samson's power.")],
    replacements: &[replacement(
        ReplacementKind::Counters,
        you_put_counters_on_your_permanent,
        that_many_plus_one,
        "puts that many plus one of each of those kinds of counters instead",
    )],
    ..oracle::DOC_SAMSON_SUPER_PSYCHIATRIST
};
