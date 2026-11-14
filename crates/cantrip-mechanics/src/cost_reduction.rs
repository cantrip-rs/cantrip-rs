//! "This spell costs {1} less to cast for each ..." and "This ability costs {1} less to activate for each ..." where the card is not
//! affinity (CR 702.41a is the keyword, and only the cards that print the word use `affinity`): a cost reduction (CR 601.2f, 118.7) that
//! counts a pool of the caster's.

use cantrip_api::*;

/// "This spell costs {1} less to cast for each instant and sorcery card in your graveyard" (CR 601.2f, 118.7a): counted as
/// the cost is determined, the cards as they are in the graveyard. Eddymurk Crab and The Dawning Archaic print it.
pub fn for_each_instant_and_sorcery_in_your_graveyard(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let cards = q.state().zone(Zone::graveyard(player)).iter().filter(|&&id| q.chars(id).is(CardType::Instant) || q.chars(id).is(CardType::Sorcery)).count();
    CostDelta::less(cards as u32)
}

/// "This spell costs {1} less to cast for each noncreature, nonland card in your graveyard" (CR 601.2f, 118.7a): counted as
/// the cost is determined, the cards as they are in the caster's graveyard; a Kindred noncreature card counts. The reduction
/// never takes off a colored symbol (CR 118.7a), which the core leaves in place. Serpent of the Pass and
/// Ravenous Cosmozoan print it.
pub fn for_each_noncreature_nonland_card_in_your_graveyard(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let cards = q.state().zone(Zone::graveyard(player)).iter().filter(|&&id| !q.chars(id).is(CardType::Creature) && !q.chars(id).is(CardType::Land)).count();
    CostDelta::less(cards as u32)
}

/// "This spell costs {N} less to cast if you control a [subtype]" (CR 601.2f, 118.7a): judged once, as the total cost is
/// determined, for the cost of casting it only (the core asks a card's own change only as it is cast), and counting only the
/// caster's permanents. `TYPE` is `lands::sub(Subtype::Kithkin)` and so on; the type is a word of the spell's rules text, read
/// through its words (CR 612.2), and a permanent has it when the layers made it that type. Mistmeadow Council (Kithkin), Visions of Villainy (Villain), Saved by the
/// Shell (Turtle), Truck Toss (Vehicle, {2}) and Geistlight Snare (Spirit) print it.
pub fn less_if_you_control_subtype<const N: u8, const TYPE: u16>(q: &dyn Query, source: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let wanted = crate::lands::word(q, source, TYPE);
    if q.permanents_of(player).into_iter().any(|id| q.chars(id).has_subtype(wanted)) { CostDelta::less(N as u32) } else { CostDelta::NONE }
}

/// "[Type] spells you cast cost {N} less to cast" (CR 601.2f, 118.7): Planar Gate
/// reduces its controller's creature spells by {2}; Shuri, Wakandan Inventor reduces artifact spells by {1}.
pub fn spells_you_cast(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, card_type: CardType, amount: u32) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player == q.controller(source) && q.chars(spell).is(card_type) { CostDelta::less(amount) } else { CostDelta::NONE }
}

/// "Noncreature spells you cast cost {`amount`} less to cast" (CR 601.2f, 118.7a): only its controller's spells, judged on the
/// spell's own characteristics as it is cast; a generic reduction takes only the generic part (CR 118.7c). Lands are never cast
/// and abilities are not spells, so neither is asked. Geist of Saint Thalia, Artist's Talent, Gran-Gran and
/// Samut, the Driving Force (`amount` is its controller's speed) print it.
pub fn noncreature_spells_you_cast(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, amount: u32) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player == q.controller(source) && !q.chars(spell).is(CardType::Creature) { CostDelta::less(amount) } else { CostDelta::NONE }
}

/// "Spells you cast cost {1} less to cast" (CR 601.2f, 118.7a): its controller's spells of any type; a generic reduction takes
/// only the generic part. Stone Calendar and The Immortal Sun.
pub fn spells_you_cast_cost_1_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(_) = costed else { return CostDelta::NONE };
    if player == q.controller(source) { CostDelta::less(1) } else { CostDelta::NONE }
}

/// Whether `spell` is the first spell `player` casts this turn that `kind` accepts (CR 601.2f, 118.7): `kind` reads a spell's
/// characteristics as it was cast, and no other such spell was cast by them this turn - before the source came to the battlefield
/// included, a spell that `kind` rejects not counting. Asked of this turn's casts, which hold a spell once it has been cast,
/// itself included. Radagast of Rhosgobel and Serah Farron ask it.
pub fn first_spell_you_cast_this_turn(q: &dyn Query, spell: ObjectId, player: PlayerId, kind: fn(&Chars) -> bool) -> bool {
    kind(&q.chars(spell)) && !q.history().casts_by(player).any(|c| c.spell != spell && kind(&c.chars(q)))
}

/// "The first [kind] spell you cast each turn costs {`amount`} less to cast" (CR 601.2f, 118.7): up to that much generic mana of its
/// total cost, after any increases; never a colored symbol, and not the mana cost or mana value of the spell. For the source's
/// controller's spells only. Radagast of Rhosgobel (creature, {2}) and Serah Farron (legendary creature, {2}) print it.
pub fn first_spell_you_cast_each_turn(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, amount: u32, kind: fn(&Chars) -> bool) -> CostDelta {
    match costed {
        Costed::Spell(spell) if player == q.controller(source) && first_spell_you_cast_this_turn(q, spell, player, kind) => CostDelta::less(amount),
        _ => CostDelta::NONE,
    }
}

/// "[Subtype] spells you cast cost {`N`} less to cast" (CR 601.2f, 118.7a): a spell of the source's controller that is that subtype as it is cast,
/// the subtype read through the words of the source's text (CR 612); generic mana only. `TYPE` is `lands::sub(Subtype::Villain)` and so on.
/// Baron Strucker, Hydra Overlord and Tombstone, Career Criminal (Villain, {1}) print it.
pub fn subtype_spells_you_cast<const N: u8, const TYPE: u16>(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player == q.controller(source) && q.chars(spell).has_subtype(crate::lands::word(q, source, TYPE)) { CostDelta::less(N as u32) } else { CostDelta::NONE }
}

/// "[Subtype] spells and [subtype] spells you cast cost {`N`} less to cast" (CR 601.2f, 118.7a): a spell of the source's controller that is
/// either subtype, or both, costs {`N`} less once (the Banneret ruling of 2024-11-08); the subtypes are read through the words of the
/// source's text (CR 612). Gagh Slinger (Klingon and Warrior) and Ballyrush Banneret (Kithkin and Soldier) print it.
pub fn either_subtype_spells_you_cast<const N: u8, const A: u16, const B: u16>(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, targets: &[Target]) -> CostDelta {
    let first = subtype_spells_you_cast::<N, A>(q, source, player, costed, targets);
    if first != CostDelta::NONE {
        return first;
    }
    subtype_spells_you_cast::<N, B>(q, source, player, costed, targets)
}

/// "[Color] spells you cast cost {1} less to cast" (CR 601.2f, 118.7a): a spell of the source's controller that is that color as it is
/// cast, the color read through the words of the source's text (CR 612); a generic reduction takes only the generic part. `COLOR` is
/// `cost_reduction::color(Color::Red)` and so on. The Fire Crystal (red), The Water Crystal (blue) and The Wind Crystal (white) print it.
pub fn colored_spells_you_cast_cost_one_less<const COLOR: u8>(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player == q.controller(source) && q.chars(spell).colors.has(q.words(source).color(Color::ALL[COLOR as usize])) { CostDelta::less(1) } else { CostDelta::NONE }
}

/// The const argument of `colored_spells_you_cast_cost_one_less`.
pub const fn color(color: Color) -> u8 {
    color as u8
}

/// "This ability costs {1} less to activate for each [subtype] you control" (CR 601.2f by way of 602.2b): the activated ability's own cost change
/// (`ActivatedDef::cost_change`), asked as it is activated and counting only the permanents of the player activating it, as the layers
/// have made them that type. Generic mana only (CostDelta::less never takes a colored symbol). `TYPE` is `lands::sub(Subtype::Island)` and so
/// on, read as a word of the source's text (CR 612.2). The Lonely Mountain (Equipment), Waterbending Scroll (Island) and Qiqirn Merchant (Town) print it.
pub fn ability_cheaper_per_subtype_you_control<const TYPE: u16>(q: &dyn Query, asker: &Asker, _: Costed, _: &[Target]) -> CostDelta {
    let wanted = crate::lands::word(q, asker.source, TYPE);
    CostDelta::less(crate::permanents_with_subtype_you_control(q, asker.controller, wanted) as u32)
}

/// "This spell costs {N} less to cast if you've cast another spell this turn" (CR 601.2f, 118.7a): asked of the event history, so a
/// countered spell or one still on the stack counts and the reduction applies once however many were cast; this spell is not yet
/// cast while its cost is worked out (CR 601.2i), so it is never among them. A generic reduction takes only the generic part
/// (CR 118.7a). Focus the Mind ({2}) and Gigastorm Titan ({3}) print it.
pub fn less_if_you_cast_another_spell_this_turn<const N: u8>(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if q.history().casts_by(player).next().is_some() { CostDelta::less(N as u32) } else { CostDelta::NONE }
}

/// "The second spell you cast each turn costs {N} less to cast" (CR 601.2f, 118.7a, 601.2i): the controller's own spells only,
/// the one being cast being their second when exactly one spell of theirs was cast earlier this turn, whoever's it was and
/// however it was cast. A generic reduction takes only the generic part (CR 118.7c). Uthros Psionicist ({2}), Highspire
/// Bell-Ringer, Monk Class and Raging Battle Mouse ({1}) print it.
pub fn second_spell_you_cast_each_turn_costs_less<const N: u8>(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if !matches!(costed, Costed::Spell(_)) || player != q.controller(source) {
        return CostDelta::NONE;
    }
    if q.history().casts_by(player).count() == 1 { CostDelta::less(N as u32) } else { CostDelta::NONE }
}
