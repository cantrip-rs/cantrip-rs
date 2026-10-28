use cantrip_api::*;
use cantrip_mechanics::{lands, targets, tokens};
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_oracle as oracle;

/// Mode 0: a +1/+1 counter on the Wolf (CR 700.2).
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::plus_one_counter_on_target(g, item)
}

static MODAL: ModalDef = modes(&[
    mode(counter, "Put a +1/+1 counter on target Wolf you control.")
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Wolf) }>)]),
    mode(tokens::create_a_treasure, "Create a Treasure token."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER, &MODAL, "Whenever this creature deals combat damage to a player, choose one \u{2014} Put a +1/+1 counter on target Wolf you control; or create a Treasure token.")],
    ..oracle::BEJEWELED_WARG
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
