use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks or becomes the target of a spell, create a Treasure token." (CR 608.2, 111.1).
fn create_treasure_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_0 = Vec::new();
    let def_0_0 = g.card_id(&tokens::TREASURE);
    tokens_0.extend(std::iter::repeat((def_0_0, item.controller)).take(1));
    g.create_tokens_worded(&tokens_0, false, None, Words::NONE)?;
    Ok(())
}

/// "Whenever this creature attacks or becomes the target of a spell" (CR 603.2).
fn when_creature_attacks_becomes_target_spell(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::BecameTarget { target: Target::Object(target), by, .. } = event else { return None };
    let id = *target;
    let by = *by;
    let subject = Target::Object(id);
    if asker.source == id
        && q.state().stack.iter().any(|i| i.kind == StackKind::Spell && i.object == by)
    {
        return Fired::about(subject);
    }
    None
}

/// "Whenever this creature attacks or becomes the target of a spell, create a Treasure token." (CR 608.2, 111.1).
fn create_treasure_token_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_2 = Vec::new();
    let def_2_0 = g.card_id(&tokens::TREASURE);
    tokens_2.extend(std::iter::repeat((def_2_0, item.controller)).take(1));
    g.create_tokens_worded(&tokens_2, false, None, Words::NONE)?;
    Ok(())
}

/// "Treasures you control have "{T}, Sacrifice this artifact: Add two mana of any one color."" (CR 604.1).
fn add_two_mana_any_one_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).has_subtype(q.words(asker.source).subtype(Subtype::Treasure))
        && asker.controller == q.controller(affected)
}

/// "Treasures you control have "{T}, Sacrifice this artifact: Add two mana of any one color."" (CR 604.1).
fn add_two_mana_any_one_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let asker = &Asker::new(source, q.controller(source));
    let card = {
        let o = q.state().obj(asker.source);
        o.copied_card().unwrap_or(o.card)
    };
    let _ = card;
    if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
        chars.abilities[slot] = Some(
            AbilityRef::delayed(
                {
                    let o = q.state().obj(asker.source);
                    o.copied_card().unwrap_or(o.card)
                },
                0,
            ),
        );
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS,
            create_treasure_token,
            "Whenever this creature attacks or becomes the target of a spell, create a Treasure token.",
        ),
        triggered(
            when_asked(
                EventMask::BECAME_TARGET,
                when_creature_attacks_becomes_target_spell,
            ),
            create_treasure_token_2,
            "Whenever this creature attacks or becomes the target of a spell, create a Treasure token.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            add_two_mana_any_one_affects,
            add_two_mana_any_one_apply_0,
            "Treasures you control have \"{T}, Sacrifice this artifact: Add two mana of any one color.\"",
        ),
    ],
    delayed: &[
        mana_ability(
            SACRIFICE_THIS.and(TAP),
            add_any_color(2),
            "{T}, Sacrifice this artifact: Add two mana of any one color.",
        ),
    ],
    ..oracle::GOLDSPAN_DRAGON
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE, &tokens::TREASURE];
