use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select, spree, targets};
use cantrip_oracle as oracle;

fn mount_or_vehicle_card(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::subtype_card_in_your_graveyard(q, asker, t, &[Subtype::Mount, Subtype::Vehicle])
}

fn aura_or_equipment_card(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::subtype_card_in_your_graveyard(q, asker, t, &[Subtype::Aura, Subtype::Equipment])
}

/// "Return target Aura or Equipment card from your graveyard to the battlefield attached to a creature you control" (CR 303.4f,
/// 303.4i, 301.5e): the creature is not a target; you choose it as this mode resolves, among the creatures you control then
/// (one a mode before it returned is there, CR 608.2c) that the card could be attached to. An Aura with none stays in the
/// graveyard (CR 303.4i); an Equipment with none enters unattached. Nothing happens if the target is no longer legal (CR 608.2b).
fn return_attached(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let you = item.controller;
    let creatures: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let hosts: Vec<ObjectId> = creatures.into_iter().filter(|&host| g.could_enter_attached(card, you, host)).collect();
    match choose::select(g, you, card, Purpose::Choose, hosts, select::creature_permanents(select::exactly(1)))?.first() {
        Some(&host) => {
            g.put_onto_battlefield_attached(card, you, host)?;
        }
        None if !g.chars(card).has_subtype(Subtype::Aura) => {
            g.put_onto_battlefield(card, you, false)?;
        }
        None => {}
    }
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(effects::return_first_target_to_battlefield, "+ {2} \u{2014} Return target creature card from your graveyard to the battlefield.")
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])
        .cost(mana("{2}")),
    mode(effects::return_first_target_to_battlefield, "+ {1} \u{2014} Return target Mount or Vehicle card from your graveyard to the battlefield.")
        .targets(&[target(mount_or_vehicle_card)])
        .cost(mana("{1}")),
    mode(return_attached, "+ {1} \u{2014} Return target Aura or Equipment card from your graveyard to the battlefield attached to a creature you control.")
        .targets(&[target(aura_or_equipment_card)])
        .cost(mana("{1}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::ONE_LAST_JOB };
