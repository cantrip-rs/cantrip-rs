use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "If you tap a basic land for mana" (CR 614.1a, 106.12b, 205.4a): a land with the basic supertype, tapped by this permanent's
/// controller, whoever's land it is.
fn basic_land_tapped_by_you(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ManaProduction(m)
        if m.player == asker.controller && q.chars(m.permanent).is(CardType::Land) && q.chars(m.permanent).supertypes.basic)
}

/// "It produces three times as much of that mana instead" (CR 614.1a, 106.12b): each type of mana the land would have produced
/// is produced three times over.
fn three_times_as_much(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    let Proposal::ManaProduction(m) = proposal else { return };
    for amount in m.mana.0.iter_mut() {
        *amount *= 3;
    }
}

/// The Adventure (CR 715).
pub static GARENBRIG_GROWTH: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_LAND_CARD_IN_YOUR_GRAVEYARD], effects::return_first_target_to_hand),
    ..oracle::GARENBRIG_GROWTH
};

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::ManaProduction,
        basic_land_tapped_by_you,
        three_times_as_much,
        "If you tap a basic land for mana, it produces three times as much of that mana instead.",
    )],
    adventure: Some(&GARENBRIG_GROWTH),
    ..oracle::VIRTUE_OF_STRENGTH
};

pub static EXTRA: &[&CardDef] = &[&GARENBRIG_GROWTH];
