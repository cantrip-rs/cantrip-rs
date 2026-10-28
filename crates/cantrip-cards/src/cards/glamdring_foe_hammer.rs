use cantrip_api::*;
use cantrip_mechanics::attach::equip;
use cantrip_mechanics::library::instant_or_sorcery_card;
use cantrip_oracle as oracle;

/// "Instant and sorcery spells you cast cost {X} less to cast, where X is equipped creature's power." (CR 601.2f, 118.7a): a
/// generic reduction of the spells its controller casts that are instants or sorceries (an Adventure's instant or sorcery half
/// included, CR 715.3), X the power the equipped creature has as the cost is determined, none being 0 (and a negative power
/// reducing nothing, CR 107.1b). The cost is locked in once determined, so the creature's power changing later alters nothing.
fn instants_and_sorceries_cost_the_equipped_creatures_power_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player != q.controller(source) || !instant_or_sorcery_card(q, spell) {
        return CostDelta::NONE;
    }
    let power = q.obj(source).attached_to.and_then(|creature| q.chars(creature).power).unwrap_or(0);
    CostDelta::less(power.clamp(0, 255) as u8)
}

/// "Mill six cards, then put all instant and sorcery cards from among them into your hand." (CR 701.17a, 701.17c, 608.2c): the
/// cards milled are the ones it takes from, as they are where they went (a graveyard, or exile by a replacement effect), every
/// instant and sorcery card of them, together.
fn gleam_of_death(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let milled = g.mill(item.controller, 6);
    g.end_sentence();
    let spells: Vec<ObjectId> = milled.into_iter().filter(|&card| instant_or_sorcery_card(g, card)).collect();
    g.return_to_hand(&spells);
    Ok(())
}

pub static GLEAM_OF_DEATH: CardDef = CardDef { spell: spell(&[], gleam_of_death), ..oracle::GLEAM_OF_DEATH };

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    cost_changes: &[instants_and_sorceries_cost_the_equipped_creatures_power_less],
    adventure: Some(&GLEAM_OF_DEATH),
    ..oracle::GLAMDRING_FOE_HAMMER
};

pub static EXTRA: &[&CardDef] = &[&GLEAM_OF_DEATH];
