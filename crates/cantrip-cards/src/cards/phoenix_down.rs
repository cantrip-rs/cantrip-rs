use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature card with mana value 4 or less from your graveyard" (CR 115.1, 202.3).
fn creature_card_with_mana_value_4_or_less_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard_with_mana_value_at_most(q, asker, t, &[CardType::Creature], 4)
}

/// "Target Skeleton, Spirit, or Zombie" (CR 115.1, 205.3): a permanent with one of those creature types, as this artifact's
/// text names them (CR 612.2).
fn skeleton_spirit_or_zombie(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Skeleton, Subtype::Spirit, Subtype::Zombie])
}

/// "Exile target Skeleton, Spirit, or Zombie."
fn exile_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile(permanent);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::return_first_target_to_battlefield_tapped, "Return target creature card with mana value 4 or less from your graveyard to the battlefield tapped.")
        .targets(&[target(creature_card_with_mana_value_4_or_less_in_your_graveyard)]),
    mode(exile_target, "Exile target Skeleton, Spirit, or Zombie.").targets(&[target(skeleton_spirit_or_zombie)]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_ability(
        cost("{1}{W}, {T}").and(EXILE_THIS),
        &MODAL,
        "{1}{W}, {T}, Exile this artifact: Choose one \u{2014} return target creature card with mana value 4 or less from your graveyard to the battlefield tapped; or exile target Skeleton, Spirit, or Zombie.",
    )],
    ..oracle::PHOENIX_DOWN
};
