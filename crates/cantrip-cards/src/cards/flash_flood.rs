use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Choose one — • Destroy target red permanent. • Return target Mountain to its owner's hand." (CR 115.1).
fn choose_one_destroy_target_red_mode_0_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).colors.has(q.words(asker.source).color(Color::Red)))
    )
}

/// "Choose one — • Destroy target red permanent. • Return target Mountain to its owner's hand." (CR 115.1).
fn choose_one_destroy_target_red_mode_1_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    cantrip_mechanics::targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Mountain])
}

static ABILITY_0_MODES: ModalDef = modes(
        &[
            mode(effects::destroy_first_target, "Destroy target red permanent.")
                .targets(&[target(choose_one_destroy_target_red_mode_0_target_0)]),
            mode(
                    effects::return_first_target_to_hand,
                    "Return target Mountain to its owner's hand.",
                )
                .targets(&[target(choose_one_destroy_target_red_mode_1_target_0)]),
        ],
    )
    .choose(1, 1);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&ABILITY_0_MODES),
    ..oracle::FLASH_FLOOD
};
