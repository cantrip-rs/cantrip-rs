use cantrip_api::*;
use cantrip_mechanics::{statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "That player gains control of Kain. If they do, you draw that many cards, create that many tapped Treasure tokens, then
/// lose that much life." The player is the one dealt the damage and "that many" the combat damage dealt (CR 510.2).
/// Kain changes hands for good (CR 611.2a). "If they do" (CR 608.2c) is what `gain_control` says: a Kain that has left the
/// battlefield, or a player who has left the game, as a lethal hit from a three-player game's Kain does (CR 800.4b), gains
/// no control, so there are no draws, no Treasure and no life loss. "You" is the ability's controller as it went on the
/// stack (CR 109.5).
fn change_hands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(player) = item.subject else { return Ok(()) };
    let kain = item.object;
    if !g.gain_control(kain, kain, player, Duration::Indefinite) {
        return Ok(());
    }
    let n = item.data[0];
    g.draw(item.controller, n);
    tokens::create_by_rule(g, item.controller, &tokens::TREASURE, n as u32, true)?;
    g.lose_life(item.controller,(n) as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, statics::itself_during_your_turn, statics::have_flying, "Jump \u{2014} During your turn, Kain has flying.")],
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT,
        change_hands,
        "Whenever Kain deals combat damage to a player, that player gains control of Kain. If they do, you draw that many cards, create that many tapped Treasure tokens, then lose that much life.",
    )],
    ..oracle::KAIN_TRAITOROUS_DRAGOON
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
