use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Spells you cast this turn that are white and/or blue cost {X} less to cast, where X is the amount of life you gained this
/// turn" (CR 611.2c, 601.2f, 608.2h): the amount is worked out as the ability resolves and kept (a later gain does not make the
/// spells cheaper). The effect is made for the player and lasts the turn whether or not Will stays on the battlefield.
fn white_and_blue_spells_cost_less_by_life_gained(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let gained = g.history().life_gained(item.controller);
    g.spells_cost_this_turn_by(item, item.controller, 0, gained);
    Ok(())
}

/// A spell that is white, blue or both (CR 105.4) costs the amount of life gained less, of generic mana only (CR 118.7a). The
/// amount is the one the effect fixed (`Query::replacement_value`).
fn white_or_blue_spells_less(q: &dyn Query, source: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let words = q.words(source);
    match costed {
        Costed::Spell(spell) if q.chars(spell).colors.has(words.color(Color::White)) || q.chars(spell).colors.has(words.color(Color::Blue)) => {
            CostDelta::less(q.replacement_value())
        }
        _ => CostDelta::NONE,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{T}"),
        white_and_blue_spells_cost_less_by_life_gained,
        "{T}: Spells you cast this turn that are white and/or blue cost {X} less to cast, where X is the amount of life you gained this turn. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    lasting_cost_changes: &[LastingCostChange {
        text: "White and/or blue spells you cast this turn cost less to cast, by the life you gained.",
        change: white_or_blue_spells_less,
        next: None,
    }],
    ..oracle::WILL_SCION_OF_PEACE
};
