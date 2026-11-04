use cantrip_api::*;
use cantrip_mechanics::{effects, pump, tokens};
use cantrip_oracle as oracle;

/// "You draw two cards and you lose 2 life. Create a 0/1 black Wizard creature token with ... Wizards you control get +1/+0 and
/// gain lifelink until end of turn." (CR 608.2c, 611.2, 611.2c): in that order, so the new token is among the Wizards the last
/// sentence affects, and no Wizard that enters later is.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    g.lose_life(item.controller, 2);
    tokens::create(g, item.controller, &tokens::WIZARD, 1)?;
    let wizard = g.words(item.object).subtype(Subtype::Wizard);
    let wizards: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(wizard)).collect();
    instruction(g, |g| {
        for id in wizards {
            pump(g, item.object, id, 1, 0);
            effects::gain_lifelink_until_end_of_turn(g, item.object, id);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::CIRCLE_OF_POWER };

pub static EXTRA: &[&CardDef] = &[&tokens::WIZARD];
