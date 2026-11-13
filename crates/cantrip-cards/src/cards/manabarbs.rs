use cantrip_api::*;
use cantrip_mechanics::{effects, mana};
use cantrip_oracle as oracle;

/// "Whenever a player taps a land for mana" (CR 603.2).
fn when_player_taps_land_mana(q: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    let (_, player, _) = mana::land_tapped_for_mana(q, event)?;
    Fired::about(Target::Player(player))
}

/// "Whenever a player taps a land for mana, this enchantment deals 1 damage to that player." (CR 608.2, 120.3).
fn enchantment_deals_1_damage_player(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    effects::damage_that_player(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::TAPPED_FOR_MANA, when_player_taps_land_mana),
            enchantment_deals_1_damage_player,
            "Whenever a player taps a land for mana, this enchantment deals 1 damage to that player.",
        ),
    ],
    ..oracle::MANABARBS
};
