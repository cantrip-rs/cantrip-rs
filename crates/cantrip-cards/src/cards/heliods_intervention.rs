use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Choose one — • Destroy X target artifacts and/or enchantments. • Target player gains twice X life." (CR 115.1).
fn choose_one_destroy_x_target_mode_0_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Enchantment)))
    )
}

/// "Choose one — • Destroy X target artifacts and/or enchantments. • Target player gains twice X life." (CR 608.2, 701.8a).
fn choose_one_destroy_x_target_mode_0(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = announced(g, item, 0, true);
    let objects_0 = t_0.iter().filter_map(|t| t.object()).collect::<Vec<ObjectId>>();
    g.destroy_all(&objects_0);
    Ok(())
}

/// "Choose one — • Destroy X target artifacts and/or enchantments. • Target player gains twice X life." (CR 115.1).
fn choose_one_destroy_x_target_mode_1_target_0(
    _: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(t, Target::Player(_))
}

/// "Choose one — • Destroy X target artifacts and/or enchantments. • Target player gains twice X life." (CR 608.2, 119.3).
fn choose_one_destroy_x_target_mode_1(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_1 = (2 * i64::from(item.x)) as i32;
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    g.gain_life(player, amount_1);
    Ok(())
}

static ABILITY_0_MODES: ModalDef = modes(
        &[
            mode(
                    choose_one_destroy_x_target_mode_0,
                    "Destroy X target artifacts and/or enchantments.",
                )
                .targets(
                    &[target(choose_one_destroy_x_target_mode_0_target_0).counted()],
                ),
            mode(choose_one_destroy_x_target_mode_1, "Target player gains twice X life.")
                .targets(&[target(choose_one_destroy_x_target_mode_1_target_0)]),
        ],
    )
    .choose(1, 1);

/// "Choose one — • Destroy X target artifacts and/or enchantments. • Target player gains twice X life." (CR 608.2).
fn announced(g: &dyn Game, item: &StackItem, spec: u8, legal: bool) -> Vec<Target> {
    (0..g.state().targets(item).len())
        .filter(|&i| {
            g.state().spec(item, i) == spec && (!legal || g.target_legal(item, i))
        })
        .map(|i| g.state().target(item, i))
        .collect()
}

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&ABILITY_0_MODES),
    ..oracle::HELIODS_INTERVENTION
};
