use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature transforms into Ashling, Rimebound and at the beginning of your first main phase": one
/// ability with two trigger events (CR 603.2).
fn transforms_or_first_main(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    triggers::this_transforms(q, source, event).or_else(|| triggers::your_first_main_phase(q, &Asker { instance: None, source, controller: q.controller(source), by: None, spell: false }, event))
}

/// "You may discard a card. If you do, draw a card."
fn rummage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_discard_to_draw(g, item, 1)
}

/// "At the beginning of your first main phase, you may pay {U}. If you do, transform Ashling."
fn pay_blue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{U}"))
}

/// "At the beginning of your first main phase, you may pay {R}. If you do, transform Ashling."
fn pay_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{R}"))
}

/// "Add two mana of any one color. Spend this mana only to cast spells with mana value 4 or greater." Not a mana ability
/// (CR 605.1b: it triggers on no mana ability), so it uses the stack and the color is chosen as it resolves (CR 106.1b).
/// The mana pays only for casting a spell whose mana value is 4 or greater on the stack, X counted (CR 106.6, 202.3e;
/// its ruling), and empties from the pool at the end of the step or phase like any mana (CR 106.4).
fn add_mana(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let color = cantrip_mechanics::choose_color_for(g, item.controller, item.object, ColorUse::AddMana)?;
    g.add_restricted_mana(item.controller, color.mana(), 2, ManaRestriction::mana_value_at_least(4));
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(when(EventMask(EventMask::TRANSFORMED.0 | EventMask::STEP_BEGAN.0), transforms_or_first_main), add_mana, "Whenever this creature transforms into Ashling, Rimebound and at the beginning of your first main phase, add two mana of any one color. Spend this mana only to cast spells with mana value 4 or greater."),
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_red, "At the beginning of your first main phase, you may pay {R}. If you do, transform Ashling."),
    ],
    ..oracle::ASHLING_RIMEBOUND
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS_OR_TRANSFORMS, rummage, "Whenever this creature enters or transforms into Ashling, Rekindled, you may discard a card. If you do, draw a card."),
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_blue, "At the beginning of your first main phase, you may pay {U}. If you do, transform Ashling."),
    ],
    back: Some(&BACK),
    ..oracle::ASHLING_REKINDLED
};
