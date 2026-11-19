use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_DIES};
use cantrip_oracle as oracle;

/// "When this creature dies, create X 1/1 black Rat creature tokens with 'This token can't block,' where X is the amount of
/// damage dealt to it this turn" (CR 603.10a, 120.2, 608.2h): every damage event this turn that had this creature as its
/// target, whoever dealt it and combat or not, summed from the event history; with none, X is 0 and nothing is created. The
/// tokens are made together (CR 111.1).
fn rats_for_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let damage: u32 = g
        .history()
        .this_turn()
        .iter()
        .filter_map(|record| match record.event {
            Event::Damage { target, amount, .. } if target == Target::Object(item.object) => Some(amount),
            _ => None,
        })
        .sum();
    tokens::create(g, item.controller, &tokens::RAT_CANT_BLOCK, damage)?;
    Ok(())
}

/// "This creature can't block" is a core keyword.
pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::CantBlock]),
    abilities: &[triggered(
        THIS_DIES,
        rats_for_damage,
        "When this creature dies, create X 1/1 black Rat creature tokens with \"This token can't block,\" where X is the amount of damage dealt to it this turn.",
    )],
    ..oracle::TANGLED_COLONY
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
