use cantrip_engine::Color;
#[path = "common/talisman_payment_state.rs"]
mod rig;

#[test]
fn payment_state_is_judged_before_untap_restores_a_blue_permanent() {
    rig::play("Malachite Talisman", "Giant Growth", Color::Red, true);
}

#[test]
fn keeping_wickermaw_blue_does_not_trigger_jihad() {
    rig::play("Malachite Talisman", "Giant Growth", Color::Blue, false);
}
