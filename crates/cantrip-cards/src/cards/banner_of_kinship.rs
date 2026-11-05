use cantrip_api::*;
use cantrip_oracle as oracle;

/// "As this artifact enters, choose a creature type. This artifact enters with a fellowship counter on it for each
/// creature you control of the chosen type" (CR 614.1c, 614.12a): the artifact itself, not yet on the battlefield, is
/// never counted.
fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let Some(chosen) = cantrip_mechanics::chosen_type::choose_creature_type(g, entering)? else { return Ok(()) };
    let count = g
        .permanents_of(entering.controller)
        .into_iter()
        .filter(|&id| {
            let c = g.chars(id);
            c.is(CardType::Creature) && c.has_subtype(chosen)
        })
        .count();
    entering.counters[CounterKind::Fellowship as usize] += i64::from(count as i16);
    Ok(())
}

/// "+1/+1 for each fellowship counter on this artifact" (CR 613.4c).
fn apply(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let bonus = q.state().obj(source).counters(CounterKind::Fellowship);
    chars.power = chars.power.map(|v| v + bonus);
    chars.toughness = chars.toughness.map(|v| v + bonus);
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters),
    statics: &[static_ability(Layer::PtModify, cantrip_mechanics::chosen_type::creatures_you_control_of_chosen_type, apply, "Creatures you control of the chosen type get +1/+1 for each fellowship counter on this artifact.")],
    ..oracle::BANNER_OF_KINSHIP
};
