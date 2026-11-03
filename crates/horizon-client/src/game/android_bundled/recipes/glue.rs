// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// The viewer's edges in the game (not in the Cargo library): the game's
// tables and events converted to the viewer's types, and what the viewer
// asks for done through the native hook points (API 2).
use crate::game::Game;
use crate::game::native_hooks::{
    NativeItem, NativePanelEvent, NativeRecipe, NativeRecipeKind, NativeRecipeLayout,
    NativeRecipeStack, NativeRecipeView, NativeStack,
};
use crate::game::official_recipes as m;

fn item(i: &NativeItem) -> m::Item {
    m::Item {
        id: i.id.clone(),
        name: i.name.clone(),
        tags: i.tags.clone(),
    }
}

fn ingredient(s: &NativeRecipeStack) -> m::Ingredient {
    m::Ingredient {
        item: s.item,
        count: s.count,
        components: s.components.clone(),
    }
}

fn recipe(r: &NativeRecipe) -> m::Recipe {
    let kind = match r.kind {
        NativeRecipeKind::CraftingShaped => m::Kind::CraftingShaped,
        NativeRecipeKind::CraftingShapeless => m::Kind::CraftingShapeless,
        NativeRecipeKind::Smelting => m::Kind::Smelting,
        NativeRecipeKind::Blasting => m::Kind::Blasting,
        NativeRecipeKind::Smoking => m::Kind::Smoking,
        NativeRecipeKind::Campfire => m::Kind::Campfire,
        NativeRecipeKind::Stonecutting => m::Kind::Stonecutting,
        NativeRecipeKind::Smithing => m::Kind::Smithing,
        NativeRecipeKind::Brewing => m::Kind::Brewing,
    };
    m::Recipe {
        kind,
        width: r.width,
        height: r.height,
        slots: r
            .slots
            .iter()
            .map(|s| s.iter().map(ingredient).collect())
            .collect(),
        results: r.results.iter().map(ingredient).collect(),
        cooking_time: r.cooking_time,
        experience: r.experience,
        book_id: r.book_id,
    }
}

fn native(s: &m::Stack) -> NativeStack {
    NativeStack {
        item: s.item.clone(),
        count: s.count,
        components: s.components.clone(),
    }
}

fn stack(s: &NativeStack) -> m::Stack {
    m::Stack {
        item: s.item.clone(),
        count: s.count,
        components: s.components.clone(),
    }
}

fn view(v: &m::View) -> NativeRecipeView {
    NativeRecipeView {
        layout: match v.layout {
            m::Layout::Grid => NativeRecipeLayout::Grid,
            m::Layout::Cooking => NativeRecipeLayout::Cooking,
            m::Layout::Single => NativeRecipeLayout::Single,
            m::Layout::Smithing => NativeRecipeLayout::Smithing,
            m::Layout::Brewing => NativeRecipeLayout::Brewing,
        },
        width: v.width,
        height: v.height,
        slots: v
            .slots
            .iter()
            .map(|s| s.iter().map(native).collect())
            .collect(),
        results: v.results.iter().map(native).collect(),
        note: v.note.clone(),
        action: v.action,
    }
}

/// Loads the world's tables once a world is open, and drops them (and the
/// panel) when it closes or the player turns the mod off.
pub(super) fn frame(game: &mut Game<'_>) {
    let tables = if game.native_on(m::ID) {
        game.native_recipes()
    } else {
        None
    };
    match (tables, m::loaded()) {
        (Some(t), false) => {
            let (viewer, out) = m::Viewer::new(
                t.items.iter().map(item).collect(),
                t.recipes.iter().map(recipe).collect(),
            );
            m::set_viewer(Some(viewer));
            apply(game, out);
        }
        (None, true) => {
            m::set_viewer(None);
            game.native_panel_recipes(m::ID, "", Vec::new(), 0, Vec::new());
            game.native_panel_items(m::ID, Vec::new());
        }
        _ => {}
    }
}

/// The player's action on the panel, as the WebAssembly mod handled it.
pub(super) fn event(game: &mut Game<'_>, event: &NativePanelEvent) {
    let event = match event {
        NativePanelEvent::Search(text) => m::Event::Search(text.clone()),
        NativePanelEvent::Clicked { index, right } => m::Event::Clicked {
            index: *index as usize,
            right: *right,
        },
        NativePanelEvent::Key { key, stack: s } => {
            let [[recipe, _, recipe_default], [uses, _, uses_default]] = m::KEYS;
            let uses = if *key == game.native_mod_key(m::ID, recipe, recipe_default) {
                false
            } else if *key == game.native_mod_key(m::ID, uses, uses_default) {
                true
            } else {
                return;
            };
            m::Event::Key {
                uses,
                stack: stack(s),
            }
        }
        NativePanelEvent::SlotClicked {
            stack: s, right, ..
        } => m::Event::SlotClicked {
            stack: stack(s),
            right: *right,
        },
        NativePanelEvent::Tab(i) => m::Event::Tab(*i as usize),
        NativePanelEvent::Action(i) => m::Event::Action(*i as usize),
        NativePanelEvent::Back => m::Event::Back,
        _ => return,
    };
    let give = m::gives(game.native_setting(m::ID, "give"));
    let known = game.native_known_recipes();
    let outs = m::with_viewer(|v| v.handle(event.clone(), give, true, &known)).unwrap_or_default();
    for out in outs {
        if let m::Out::Give { item, count } = &out {
            // Not in creative mode: the click shows the item's page, as with the setting off.
            if game.native_give(item, *count).is_err() {
                let again = m::with_viewer(|v| v.handle(event.clone(), false, false, &known));
                for out in again.unwrap_or_default() {
                    apply(game, out);
                }
            }
            continue;
        }
        apply(game, out);
    }
}

fn apply(game: &mut Game<'_>, out: m::Out) {
    match out {
        m::Out::Items(items) => game.native_panel_items(m::ID, items.iter().map(native).collect()),
        m::Out::Recipes {
            title,
            tabs,
            tab,
            views,
        } => game.native_panel_recipes(
            m::ID,
            &title,
            tabs.iter().map(native).collect(),
            tab as u32,
            views.iter().map(view).collect(),
        ),
        m::Out::Give { item, count } => {
            let _ = game.native_give(&item, count);
        }
        m::Out::Place(book) => {
            if let Err(e) = game.native_place_recipe(book, false) {
                game.native_chat(&e);
            }
        }
    }
}
