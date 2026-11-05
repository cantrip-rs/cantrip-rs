use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::job_select::JOB_SELECT;
use cantrip_mechanics::tokens::HERO_1_1_COLORLESS;
use cantrip_mechanics::statics::{also_a, grant_delayed_ability, plus_pt};
use cantrip_mechanics::{effects, hand, lands, triggers};
use cantrip_oracle as oracle;

/// "Draw a card, then discard a card. That player loses life equal to the discarded card's mana value" (CR 121.1, 701.9, 119.3):
/// the card is read as it last existed in the graveyard it went to, so {X} counts as 0 (CR 202.3e); nothing discarded (an empty
/// hand, or a discard that put no card anywhere) is no life lost. The player is the one the creature dealt damage to.
fn loot_and_drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let discarded = effects::loot_discarding(g, item.controller, item.object)?;
    let Some(player) = item.subject.player() else { return Ok(()) };
    for card in discarded {
        if hand::discarded_chars(&*g, card).is_some() {
            let value = i64::from(g.mana_value(card));
            g.lose_life(player, value);
        }
    }
    Ok(())
}

const TEXT: &str = "Whenever this creature deals combat damage to a player, draw a card, then discard a card. That player loses life equal to the discarded card's mana value.";

pub static CARD: CardDef = CardDef {
    abilities: &[JOB_SELECT, equip(mana("{2}"), "Mutsunokami \u{2014} Equip {2}")],
    delayed: &[triggered(triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT, loot_and_drain, TEXT)],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Type, attached_object, also_a::<{ lands::sub(Subtype::Ninja) }>, "Equipped creature is a Ninja in addition to its other types."),
        static_ability(
            Layer::Ability,
            attached_object,
            grant_delayed_ability,
            "Equipped creature has \"Whenever this creature deals combat damage to a player, draw a card, then discard a card. That player loses life equal to the discarded card's mana value.\"",
        )
        .continues(),
    ],
    ..oracle::NINJAS_BLADES
};

pub static EXTRA: &[&CardDef] = &[&HERO_1_1_COLORLESS];
