use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "As long as Gaea's Liege isn't attacking, its power and toughness are each equal to the number of Forests you control. As long as Gaea's Liege is attacking, its power and toughness are each equal to the number of Forests defending player controls." (CR 604.1).
fn affects(_: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected
}

fn set_power_toughness(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let player = q.state().attackers.iter().find(|a| a.id == source)
        .map_or_else(|| q.controller(source), |a| a.defending);
    let forest = q.words(source).subtype(Subtype::Forest);
    let count = q.state().battlefield().iter()
        .filter(|&&id| q.controller(id) == player && q.chars(id).has_subtype(forest))
        .count() as i32;
    chars.power = Some(count);
    chars.toughness = Some(count);
}

/// "{T}: Target land becomes a Forest until this creature leaves the battlefield." (CR 608.2, 611.2a).
fn make_forest(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let forest = g.words(item.object).subtype(Subtype::Forest);
    g.add_effect(item.object, target, EffectKind::SetLandType(forest), Duration::WhileSourceOnBattlefield);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, make_forest, "{T}: Target land becomes a Forest until this creature leaves the battlefield.")
        .targets(&[targets::LAND])],
    statics: &[static_ability(Layer::PtSet, affects, set_power_toughness,
        "As long as Gaea's Liege isn't attacking, its power and toughness are each equal to the number of Forests you control. As long as Gaea's Liege is attacking, its power and toughness are each equal to the number of Forests defending player controls.")],
    ..oracle::GAEAS_LIEGE
};
