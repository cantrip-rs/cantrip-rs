use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "If a red source you control would deal an amount of noncombat damage less than Ojer Axonil's power to an opponent"
/// (CR 614.1a, 120.3): a source its controller controls and that is red as it would deal the damage (a spell, an ability's
/// source, a permanent), noncombat damage to an opponent - a player, not a permanent they control - and an amount, after
/// what has already applied, below Ojer Axonil's power as the damage would be dealt (the ruling: compared once, then).
/// Damage prevented to nothing is no damage to raise (CR 615.6).
fn noncombat_damage_below_power(q: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    let Proposal::Damage(d) = p else { return false };
    let Target::Player(player) = d.target else { return false };
    let power = q.chars(asker.source).power.unwrap_or(0);
    d.amount > 0
        && !d.combat
        && i64::from(d.amount) < i64::from(power)
        && q.controller(d.source) == asker.controller
        && q.chars(d.source).colors.has(q.words(asker.source).color(Color::Red))
        && q.state().opponents(asker.controller).any(|opponent| opponent == player)
}

/// "That source deals damage equal to Ojer Axonil's power instead."
fn equal_to_power(q: &dyn Query, asker: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount = q.chars(asker.source).power.unwrap_or(0).max(0) as u32;
    }
}

/// "Activate only if red sources you controlled dealt 4 or more noncombat damage this turn": this turn's damage events,
/// each with the source's controller and colors as it dealt the damage (the ruling: a source that has since died or
/// changed control still counts), to any player or permanent, added up across sources.
fn red_sources_dealt_four_noncombat_damage(q: &dyn Query, asker: &Asker) -> bool {
    let red = q.words(asker.source).color(Color::Red);
    let dealt: i64 = q
        .history()
        .this_turn()
        .iter()
        .filter_map(|r| match r.event {
            Event::Damage { amount, kind, controller, .. } if !kind.is_combat() && controller == asker.controller && kind.colors().has(red) => {
                Some(i64::from(amount))
            }
            _ => None,
        })
        .sum();
    dealt >= 4
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
        ability(cost("{2}{R}, {T}"), effects::transform_self, "{2}{R}, {T}: Transform this land. Activate only if red sources you controlled dealt 4 or more noncombat damage this turn and only as a sorcery.")
            .sorcery_speed()
            .condition(red_sources_dealt_four_noncombat_damage),
    ],
    ..oracle::TEMPLE_OF_POWER
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_DIES, effects::return_it_tapped_and_transformed, "When Ojer Axonil dies, return it to the battlefield tapped and transformed under its owner's control."),
    ],
    replacements: &[replacement(
        ReplacementKind::Damage,
        noncombat_damage_below_power,
        equal_to_power,
        "deals damage equal to Ojer Axonil's power instead",
    )],
    back: Some(&BACK),
    ..oracle::OJER_AXONIL_DEEPEST_MIGHT
};
