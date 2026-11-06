use cantrip_api::*;
use cantrip_mechanics::ninjutsu::ninjutsu;
use cantrip_oracle as oracle;

/// That player exiles the top two cards of their library, one at a time. Until end of turn, the controller may play
/// those cards (lands too, CR 305.1) without paying their mana costs. A card a replacement effect sent elsewhere is
/// not "those cards".
fn plunder(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(victim) = item.subject else { return Ok(()) };
    let grant = PlayGrant { instead: Some(ManaCost::ZERO), lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST };
    cantrip_mechanics::library::exile_top_may_play(g, victim, 2, item.controller, grant);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ninjutsu(
            mana("{2}{U}{B}"),
            "Ninjutsu {2}{U}{B} ({2}{U}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)",
        ),
        triggered(cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT, plunder, "Whenever this creature deals combat damage to a player, that player exiles the top two cards of their library. Until end of turn, you may play those cards without paying their mana costs."),
    ],
    ..oracle::FALLEN_SHINOBI
};
