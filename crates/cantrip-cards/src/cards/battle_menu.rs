use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

fn attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::KNIGHT_2_2_WHITE, 1)?;
    Ok(())
}

/// Choose one mode as the spell is cast (CR 700.2a); only Ability and Magic have a target (CR 700.2c).
static MODAL: ModalDef = modes(&[
    mode(attack, "Attack — Create a 2/2 white Knight creature token."),
    mode(effects::target_creature_gets::<0, 4>, "Ability — Target creature gets +0/+4 until end of turn.").targets(&[targets::CREATURE]),
    mode(effects::destroy_first_target, "Magic — Destroy target creature with power 4 or greater.").targets(&[targets::CREATURE_WITH_POWER_4_OR_GREATER]),
    mode(|g, item| effects::gain_life(g, item, 4), "Item — You gain 4 life."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::BATTLE_MENU };

pub static EXTRA: &[&CardDef] = &[&tokens::KNIGHT_2_2_WHITE];
