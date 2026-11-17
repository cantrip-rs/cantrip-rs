use cantrip_api::*;
use cantrip_mechanics::{outlaw, spree};
use cantrip_oracle as oracle;

/// "Caught in the Crossfire deals 2 damage to each outlaw creature" / "... each non-outlaw creature" (CR 700.12, 608.2c, 120.2):
/// every creature on the battlefield, as the effect is carried out, that is (or is not) an outlaw, as the layers made it; all of
/// the damage is dealt together, one event.
fn two_damage_to_each_creature<const OUTLAWS: bool>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, u32)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let chars = g.chars(id);
            chars.is(CardType::Creature) && outlaw::is_outlaw(&chars) == OUTLAWS
        })
        .map(|id| (item.object, Target::Object(id), 2))
        .collect();
    g.damage_together(&hits);
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(
        two_damage_to_each_creature::<true>,
        "+ {1} \u{2014} Caught in the Crossfire deals 2 damage to each outlaw creature. (Assassins, Mercenaries, Pirates, Rogues, and Warlocks are outlaws.)",
    )
    .cost(mana("{1}")),
    mode(two_damage_to_each_creature::<false>, "+ {1} \u{2014} Caught in the Crossfire deals 2 damage to each non-outlaw creature.")
        .cost(mana("{1}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::CAUGHT_IN_THE_CROSSFIRE };
