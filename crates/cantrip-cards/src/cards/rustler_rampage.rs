use cantrip_api::*;
use cantrip_mechanics::{effects, spree, targets};
use cantrip_oracle as oracle;

/// "Untap all creatures target player controls" (CR 701.26b): every creature that player controls as this resolves, with no
/// choice, untapped together (CR 101.4); nothing if the player is no longer a legal target (CR 608.2b).
fn untap_creatures_target_player_controls(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    targets::player(g, item, 0).into_iter().for_each(|player| {
        let creatures: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
        g.untap_together(&creatures);
    });
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(untap_creatures_target_player_controls, "+ {1} \u{2014} Untap all creatures target player controls.").targets(&[targets::PLAYER]).cost(mana("{1}")),
    mode(effects::target_gains_double_strike, "+ {1} \u{2014} Target creature gains double strike until end of turn.").targets(&[targets::CREATURE]).cost(mana("{1}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    ..oracle::RUSTLER_RAMPAGE
};
