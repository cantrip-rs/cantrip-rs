//! Affinity (CR 702.41a): affinity for artifacts (Emry, Lurker of the Loch and Thoughtcast, and Sami, Wildcat Captain's spells you cast) and for a basic land type
//! (Sapling Nursery's Forests), a creature type (Scales of Shale's Lizards), creatures (Witherbloom, the Balancer) and
//! artifact creatures (Borg Cube, Urza, Chief Artificer), outlaws (Hellspur Brute) and planeswalkers (Tomik, Wielder of Law).

use cantrip_api::*;

/// {1} less for each permanent `player` controls that `counts`. Asked of a spell only: the core asks a card's own changes
/// only as it is cast, and `instants_and_sorceries_you_cast_have_affinity_for_creatures` only of a spell.
fn less_for_each_permanent(q: &dyn Query, player: PlayerId, counts: impl Fn(ObjectId) -> bool) -> CostDelta {
    let permanents = q.permanents_of(player).into_iter().filter(|&id| counts(id)).count();
    CostDelta::less(permanents as u32)
}

/// This spell costs {1} less to cast for each artifact you control.
pub fn for_artifacts(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    less_for_each_permanent(q, player, |id| q.chars(id).is(CardType::Artifact))
}

/// "Spells you cast have affinity for artifacts" (CR 702.41a, 702.41b, 604.1): a static ability of the source, asked as each
/// spell is cast; only its controller's spells, every kind, one reduction for each source (two of them reduce twice, CR
/// 702.41b), the generic part only (CR 118.7c). Sami, Wildcat Captain prints it, as a cost change in `CardDef::cost_changes`.
pub fn spells_you_cast_have_affinity_for_artifacts(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if !matches!(costed, Costed::Spell(_)) || player != q.controller(source) {
        return CostDelta::NONE;
    }
    for_artifacts(q, source, player, costed, &[])
}

/// A spell costs {1} less to cast for each artifact creature you control (CR 702.41a). Borg Cube and Urza, Chief Artificer
/// print affinity for artifact creatures.
pub fn for_artifact_creatures(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    less_for_each_permanent(q, player, |id| {
        let c = q.chars(id);
        c.is(CardType::Artifact) && c.is(CardType::Creature)
    })
}

/// "Affinity for [basic land type]es" (CR 702.41a): this spell costs {1} less to cast for each land of that type you control,
/// `TYPE` being `lands::sub(Subtype::Forest)` and so on. The type is a word of the spell's rules text, read through its words
/// (CR 612.2), and a land has it when the layers made it that type (a Forest is any land with the subtype, basic or not).
/// Sapling Nursery prints affinity for Forests.
pub fn for_land_type<const TYPE: u16>(q: &dyn Query, source: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let wanted = crate::lands::word(q, source, TYPE);
    let lands = q.permanents_of(player).into_iter().filter(|&id| q.chars(id).is(CardType::Land) && q.chars(id).subtypes.has(wanted)).count();
    CostDelta::less(lands as u32)
}

/// "Affinity for [creature type]s" (CR 702.41a): this spell costs {1} less to cast for each permanent of that type you
/// control, `TYPE` being `lands::sub(Subtype::Lizard)` and so on. The type is a word of the spell's rules text, read through
/// its words (CR 612.2), and a permanent has it when the layers made it that type (a changeling has every creature type).
/// Scales of Shale prints affinity for Lizards.
pub fn for_creature_type<const TYPE: u16>(q: &dyn Query, source: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let wanted = crate::lands::word(q, source, TYPE);
    let creatures = q.permanents_of(player).into_iter().filter(|&id| q.chars(id).has_subtype(wanted)).count();
    CostDelta::less(creatures as u32)
}

/// "Affinity for creatures" (CR 702.41a): this spell costs {1} less to cast for each creature you control. Witherbloom, the
/// Balancer prints it for itself, and `instants_and_sorceries_you_cast` gives it to the spells of the same controller.
pub fn for_creatures(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    less_for_each_permanent(q, player, |id| q.chars(id).is(CardType::Creature))
}

/// "Instant and sorcery spells you cast have affinity for creatures" (CR 702.41a, 702.41b, 604.1): a static ability of the
/// source, asked as each spell is cast; only its controller's instants and sorceries, one reduction for each source (two of them
/// reduce twice, CR 702.41b), the generic part only (CR 118.7c). Witherbloom, the Balancer prints it, as a cost change in
/// `CardDef::cost_changes`.
pub fn instants_and_sorceries_you_cast_have_affinity_for_creatures(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    let chars = q.chars(spell);
    if player != q.controller(source) || !(chars.is(CardType::Instant) || chars.is(CardType::Sorcery)) {
        return CostDelta::NONE;
    }
    for_creatures(q, source, player, costed, &[])
}

/// "Affinity for outlaws" (CR 702.41a, 700.12): this spell costs {1} less to cast for each outlaw its caster controls, as it is
/// cast (the generic part only, CR 118.7c). Outlaws are permanents of the five types the term names, as the layers made them.
/// Hellspur Brute prints it.
pub fn for_outlaws(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(crate::outlaw::outlaws_controlled_by(q, player) as u32)
}

/// "Affinity for planeswalkers" (CR 702.41a): this spell costs {1} less to cast for each planeswalker you control. Tomik, Wielder
/// of Law prints it.
pub fn for_planeswalkers(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    less_for_each_permanent(q, player, |id| q.chars(id).is(CardType::Planeswalker))
}
