use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "When Gwyndala enters, create a colorless Equipment artifact token named Neuroflux Heirloom with \"Equipped creature
/// gets +1/+1\" and equip {2}. Attach it to Gwyndala." The created token is attached if Gwyndala is still a creature
/// when this resolves (CR 301.5e, 701.3b).
fn create_heirloom_and_attach(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create(g, item.controller, &NEUROFLUX_HEIRLOOM, 1)?;
    if let Some(&heirloom) = made.first() {
        g.attach(heirloom, item.object);
    }
    Ok(())
}

/// "Each Equipment token you control becomes a copy of target Equipment you control until end of turn." The target's
/// copiable values are read once as this resolves (CR 707.2); all Equipment tokens controlled now receive that same copy
/// effect, which ends in cleanup (CR 611.2c, 514.2). The effect does not copy attachments, counters, or other effects.
fn equipment_tokens_become_copies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(target);
    let equipment_tokens: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.obj(id).token && g.chars(id).has_subtype(Subtype::Equipment))
        .collect();
    instruction(g, |g| {
        for token in equipment_tokens {
            g.become_copy(item.object, token, values, Duration::EndOfTurn);
        }
    });
    Ok(())
}

static NEUROFLUX_HEIRLOOM: CardDef = CardDef {
    name: "Neuroflux Heirloom",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    abilities: &[attach::equip(mana("{2}"), "Equip {2}")],
    statics: &[static_ability(Layer::PtModify, attach::attached_object, statics::plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    text: "Equipped creature gets +1/+1.\nEquip {2}",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            create_heirloom_and_attach,
            "When Gwyndala enters, create a colorless Equipment artifact token named Neuroflux Heirloom with \"Equipped creature gets +1/+1\" and equip {2}. Attach it to Gwyndala.",
        ),
        ability(cost("{2}"), equipment_tokens_become_copies, "{2}: Each Equipment token you control becomes a copy of target Equipment you control until end of turn.")
            .targets(&[targets::EQUIPMENT_YOU_CONTROL]),
    ],
    ..oracle::GWYNDALA_THE_PROGENY
};

pub static EXTRA: &[&CardDef] = &[&NEUROFLUX_HEIRLOOM];
