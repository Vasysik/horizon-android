// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// Horizon's official recipe viewer, formerly the WebAssembly mod: every item
// in the panel the game draws beside container screens, with a search box. R
// over an item, the panel's or any slot's, shows its recipes, U its uses; a
// click does the same, left for recipes and right for uses. A recipe page has
// a tab per station, and + beside a crafting recipe the player knows fills the
// grid as the recipe book does. In creative mode, with "Click gives items" on,
// a click on a panel item gives one, a right click a stack. Its setting and
// keys are the WebAssembly mod's (`horizon-mods/.horizon-mod-data/recipes/give`,
// `key.recipes.recipe` and `key.recipes.uses` in horizon-mods.txt), so a
// player keeps them. This file is both the mod's Cargo library (its tests) and
// the module the bundle adds to the game; the game's types are converted at
// the edges (frame.rs, event.rs).

/// The mod's id (its settings folder), name and version.
pub const ID: &str = "recipes";
pub const NAME: &str = "Recipes";
pub const VERSION: &str = "2.0.0";
pub const DESCRIPTION: &str = "Every item beside your inventory, with a search box. R over an item shows its recipes, U its uses, and + fills the crafting grid with a recipe you know. Horizon's own.";
/// Its keys: name, label, default (the WebAssembly mod's).
pub const KEYS: [[&str; 3]; 2] = [
    ["recipe", "Show recipes", "key.keyboard.r"],
    ["uses", "Show uses", "key.keyboard.u"],
];
/// What a recipe page holds at most, as the panel takes.
pub const MAX_RECIPES: usize = 256;
pub const MAX_STACKS: usize = 64;
/// A right click's gift in creative mode (cut to one stack by the game).
pub const STACK: u32 = 99;
/// The stations, in the order of their tabs, with each one's icon.
pub const STATIONS: [&str; 8] = [
    "minecraft:crafting_table",
    "minecraft:furnace",
    "minecraft:blast_furnace",
    "minecraft:smoker",
    "minecraft:campfire",
    "minecraft:stonecutter",
    "minecraft:smithing_table",
    "minecraft:brewing_stand",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    CraftingShaped,
    CraftingShapeless,
    Smelting,
    Blasting,
    Smoking,
    Campfire,
    Stonecutting,
    Smithing,
    Brewing,
}

fn station(kind: Kind) -> usize {
    match kind {
        Kind::CraftingShaped | Kind::CraftingShapeless => 0,
        Kind::Smelting => 1,
        Kind::Blasting => 2,
        Kind::Smoking => 3,
        Kind::Campfire => 4,
        Kind::Stonecutting => 5,
        Kind::Smithing => 6,
        Kind::Brewing => 7,
    }
}

/// An item the game knows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
}

/// An item of the tables by its index, as recipes name them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ingredient {
    pub item: u32,
    pub count: u32,
    pub components: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    pub kind: Kind,
    pub width: u32,
    pub height: u32,
    pub slots: Vec<Vec<Ingredient>>,
    pub results: Vec<Ingredient>,
    pub cooking_time: u32,
    pub experience: f32,
    /// The recipe book's id, or -1.
    pub book_id: i32,
}

/// An item as the panel shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stack {
    pub item: String,
    pub count: u32,
    pub components: String,
}

fn stack(item: &str) -> Stack {
    Stack {
        item: item.into(),
        count: 1,
        components: String::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Grid,
    Cooking,
    Single,
    Smithing,
    Brewing,
}

/// A recipe as the panel lays it out.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub layout: Layout,
    pub width: u32,
    pub height: u32,
    pub slots: Vec<Vec<Stack>>,
    pub results: Vec<Stack>,
    pub note: String,
    /// + beside it: none, or whether the book holds it.
    pub action: Option<bool>,
}

/// What the player did on the panel.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Search(String),
    /// A panel item, by its index in the items shown.
    Clicked {
        index: usize,
        right: bool,
    },
    /// The recipes key (`uses` false) or the uses key over an item.
    Key {
        uses: bool,
        stack: Stack,
    },
    /// A slot of a recipe shown.
    SlotClicked {
        stack: Stack,
        right: bool,
    },
    Tab(usize),
    Action(usize),
    Back,
}

/// What the game is to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    /// Show these items in the panel.
    Items(Vec<Stack>),
    /// Show this recipe page; an empty one goes back to the items.
    Recipes {
        title: String,
        tabs: Vec<Stack>,
        tab: usize,
        views: Vec<View>,
    },
    /// Creative mode: give the player this item.
    Give { item: String, count: u32 },
    /// Place this recipe book id in the open grid.
    Place(i32),
}

/// A recipe page: an item's recipes or its uses, on one station's tab.
#[derive(Clone, Debug, PartialEq)]
struct Page {
    item: Stack,
    uses: bool,
    tab: usize,
}

/// The viewer's state while a world is open.
#[derive(Default)]
pub struct Viewer {
    items: Vec<Item>,
    all: Vec<Recipe>,
    /// The panel's items now: indexes into `items`, after the search.
    listed: Vec<usize>,
    search: String,
    /// The page shown, then the ones Back returns to.
    history: Vec<Page>,
    /// The book id behind each recipe the page shows (-1: none).
    shown: Vec<i32>,
}

impl Viewer {
    /// The world's tables: the panel lists every item.
    pub fn new(items: Vec<Item>, all: Vec<Recipe>) -> (Self, Out) {
        let mut me = Viewer {
            items,
            all,
            ..Viewer::default()
        };
        let out = me.list();
        (me, out)
    }

    fn id(&self, s: &Ingredient) -> &str {
        self.items
            .get(s.item as usize)
            .map_or("minecraft:air", |i| &i.id)
    }
    fn convert(&self, s: &Ingredient) -> Stack {
        Stack {
            item: self.id(s).into(),
            count: s.count,
            components: s.components.clone(),
        }
    }
    /// The same item, and the same components when `want` names some.
    fn matches(&self, s: &Ingredient, want: &Stack) -> bool {
        self.id(s) == want.item && (want.components.is_empty() || s.components == want.components)
    }
    fn name<'a>(&'a self, item: &'a str) -> &'a str {
        self.items
            .iter()
            .find(|i| i.id == item)
            .map_or(item, |i| &i.name)
    }

    /// The items the search keeps: every word must be in the name or the
    /// id; `@minecraft` keeps a namespace, `#logs` a tag.
    fn list(&mut self) -> Out {
        let words: Vec<String> = self
            .search
            .to_lowercase()
            .split_whitespace()
            .map(String::from)
            .collect();
        let keeps = |item: &Item| {
            words.iter().all(|w| {
                if let Some(ns) = w.strip_prefix('@') {
                    item.id.split(':').next().is_some_and(|n| n.starts_with(ns))
                } else if let Some(tag) = w.strip_prefix('#') {
                    item.tags.iter().any(|t| t.contains(tag))
                } else {
                    item.name.to_lowercase().contains(w.as_str()) || item.id.contains(w.as_str())
                }
            })
        };
        self.listed = (self.items.iter().enumerate())
            .filter(|(_, i)| i.id != "minecraft:air" && keeps(i))
            .map(|(n, _)| n)
            .collect();
        Out::Items(
            self.listed
                .iter()
                .map(|&n| stack(&self.items[n].id))
                .collect(),
        )
    }

    /// The recipes a page lists, all stations together.
    fn found(&self, page: &Page) -> Vec<&Recipe> {
        (self.all.iter())
            .filter(|r| {
                if page.uses {
                    r.slots
                        .iter()
                        .flatten()
                        .any(|s| self.matches(s, &page.item))
                } else {
                    r.results.iter().any(|s| self.matches(s, &page.item))
                }
            })
            .collect()
    }

    /// Opens a page, pushing it on the history; nothing when it is empty.
    fn open(&mut self, item: Stack, uses: bool, known: &[i32]) -> Option<Out> {
        let item = Stack { count: 1, ..item };
        let page = Page { item, uses, tab: 0 };
        if self.found(&page).is_empty() {
            return None;
        }
        self.history.push(page);
        Some(self.show(known))
    }

    /// The page on top of the history, or no page when there is none.
    fn show(&mut self, known: &[i32]) -> Out {
        let Some(mut page) = self.history.last().cloned() else {
            self.shown.clear();
            return Out::Recipes {
                title: String::new(),
                tabs: Vec::new(),
                tab: 0,
                views: Vec::new(),
            };
        };
        let found = self.found(&page);
        let stations: Vec<usize> = (0..STATIONS.len())
            .filter(|&s| found.iter().any(|r| station(r.kind) == s))
            .collect();
        page.tab = page.tab.min(stations.len().saturating_sub(1));
        let on_tab = |r: &&&Recipe| stations.get(page.tab) == Some(&station(r.kind));
        let on: Vec<&Recipe> = found
            .iter()
            .filter(on_tab)
            .copied()
            .take(MAX_RECIPES)
            .collect();
        let total = found.iter().filter(on_tab).count();
        let views = on.iter().map(|r| self.view(r, known)).collect();
        let mut title = format!(
            "{}: {}",
            if page.uses { "Uses" } else { "Recipes" },
            self.name(&page.item.item)
        );
        if total > on.len() {
            title.push_str(&format!(" ({} of {total})", on.len()));
        }
        let tabs = stations.iter().map(|&s| stack(STATIONS[s])).collect();
        self.shown = on.iter().map(|r| r.book_id).collect();
        if let Some(top) = self.history.last_mut() {
            top.tab = page.tab;
        }
        Out::Recipes {
            title,
            tabs,
            tab: page.tab,
            views,
        }
    }

    fn view(&self, r: &Recipe, known: &[i32]) -> View {
        let mut slots: Vec<Vec<Stack>> = (r.slots.iter())
            .map(|s| s.iter().take(MAX_STACKS).map(|s| self.convert(s)).collect())
            .collect();
        let results = (r.results.iter().take(MAX_STACKS))
            .map(|s| self.convert(s))
            .collect();
        let (layout, width, height, note) = match r.kind {
            Kind::CraftingShaped => (Layout::Grid, r.width, r.height, String::new()),
            Kind::CraftingShapeless => {
                let n = slots.len() as u32;
                let width = match n {
                    0 | 1 => 1,
                    2..=4 => 2,
                    _ => 3,
                };
                let height = n.div_ceil(width).max(1);
                slots.resize((width * height) as usize, Vec::new());
                (Layout::Grid, width, height, "Shapeless".into())
            }
            Kind::Smelting | Kind::Blasting | Kind::Smoking | Kind::Campfire => {
                let seconds = r.cooking_time as f32 / 20.0;
                let note = format!("{seconds} s, {} XP", (r.experience * 100.0).round() / 100.0);
                (Layout::Cooking, 1, 1, note)
            }
            Kind::Stonecutting => (Layout::Single, 1, 1, String::new()),
            Kind::Smithing => (Layout::Smithing, 3, 1, String::new()),
            Kind::Brewing => (Layout::Brewing, 2, 1, String::new()),
        };
        // + only for what the recipe book shows; greyed until the book has it.
        let action = (r.book_id >= 0).then(|| known.contains(&r.book_id));
        View {
            layout,
            width,
            height,
            slots,
            results,
            note,
            action,
        }
    }

    /// The player's action; `give` is the setting, `creative` whether the
    /// game would give. `known` is the recipe book's ids now.
    pub fn handle(&mut self, event: Event, give: bool, creative: bool, known: &[i32]) -> Vec<Out> {
        match event {
            Event::Search(text) => {
                self.search = text;
                vec![self.list()]
            }
            Event::Clicked { index, right } => {
                let Some(&n) = self.listed.get(index) else {
                    return Vec::new();
                };
                let item = self.items[n].id.clone();
                if give && creative {
                    let count = if right { STACK } else { 1 };
                    return vec![Out::Give { item, count }];
                }
                self.open(stack(&item), right, known).into_iter().collect()
            }
            Event::Key { uses, stack } => self.open(stack, uses, known).into_iter().collect(),
            Event::SlotClicked { stack, right } => {
                self.open(stack, right, known).into_iter().collect()
            }
            Event::Tab(i) => {
                if let Some(top) = self.history.last_mut() {
                    top.tab = i;
                }
                vec![self.show(known)]
            }
            Event::Action(i) => match self.shown.get(i) {
                Some(&book) if book >= 0 => vec![Out::Place(book)],
                _ => Vec::new(),
            },
            Event::Back => {
                self.history.pop();
                vec![self.show(known)]
            }
        }
    }
}

/// "Click gives items" from its stored value (the WebAssembly mod's
/// `true`/`false`); off by default.
pub fn gives(stored: Option<String>) -> bool {
    stored.is_some_and(|v| v.trim() == "true")
}

/// The settings page's button.
pub fn give_label(give: bool) -> String {
    format!(
        "Click gives items (creative): {}",
        if give { "ON" } else { "OFF" }
    )
}

std::thread_local! {
    /// The viewer while a world is open; the game is one thread.
    static VIEWER: std::cell::RefCell<Option<Viewer>> = const { std::cell::RefCell::new(None) };
}

/// Runs `f` on the viewer, if a world is open.
pub fn with_viewer<R>(f: impl FnOnce(&mut Viewer) -> R) -> Option<R> {
    VIEWER.with(|v| v.borrow_mut().as_mut().map(f))
}

/// Whether the viewer holds a world's tables.
pub fn loaded() -> bool {
    VIEWER.with(|v| v.borrow().is_some())
}

/// Keeps the viewer of a world just opened (or none when it closed).
pub fn set_viewer(viewer: Option<Viewer>) {
    VIEWER.with(|v| *v.borrow_mut() = viewer);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables() -> (Vec<Item>, Vec<Recipe>) {
        let item = |id: &str, name: &str, tags: &[&str]| Item {
            id: id.into(),
            name: name.into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
        };
        let items = vec![
            item("minecraft:air", "Air", &[]),
            item("minecraft:oak_log", "Oak Log", &["minecraft:logs"]),
            item("minecraft:oak_planks", "Oak Planks", &["minecraft:planks"]),
            item("minecraft:stick", "Stick", &[]),
            item("minecraft:charcoal", "Charcoal", &[]),
        ];
        let one = |item| Ingredient {
            item,
            count: 1,
            components: String::new(),
        };
        let planks = Recipe {
            kind: Kind::CraftingShapeless,
            width: 0,
            height: 0,
            slots: vec![vec![one(1)]],
            results: vec![Ingredient { count: 4, ..one(2) }],
            cooking_time: 0,
            experience: 0.0,
            book_id: 7,
        };
        let sticks = Recipe {
            kind: Kind::CraftingShaped,
            width: 1,
            height: 2,
            slots: vec![vec![one(2)], vec![one(2)]],
            results: vec![Ingredient { count: 4, ..one(3) }],
            book_id: 8,
            ..planks.clone()
        };
        let charcoal = Recipe {
            kind: Kind::Smelting,
            width: 0,
            height: 0,
            slots: vec![vec![one(1)]],
            results: vec![one(4)],
            cooking_time: 200,
            experience: 0.15,
            book_id: 9,
        };
        (items, vec![planks, sticks, charcoal])
    }

    fn ids(out: &Out) -> Vec<&str> {
        match out {
            Out::Items(items) => items.iter().map(|s| s.item.as_str()).collect(),
            _ => panic!("{out:?}"),
        }
    }

    #[test]
    fn lists_every_item_but_air_and_searches_names_ids_namespaces_and_tags() {
        let (items, all) = tables();
        let (mut v, out) = Viewer::new(items, all);
        assert_eq!(ids(&out).len(), 4);
        let search = |v: &mut Viewer, text: &str| {
            ids(&v.handle(Event::Search(text.into()), false, false, &[])[0]).len()
        };
        assert_eq!(search(&mut v, "oak"), 2);
        assert_eq!(search(&mut v, "#logs"), 1);
        assert_eq!(search(&mut v, "@mine stick"), 1);
        assert_eq!(search(&mut v, "@other"), 0);
    }

    #[test]
    fn a_click_shows_recipes_and_a_right_click_uses_with_a_tab_per_station() {
        let (items, all) = tables();
        let (mut v, _) = Viewer::new(items, all);
        // Oak Log is listed first: it has no recipe, so nothing opens.
        assert!(
            v.handle(
                Event::Clicked {
                    index: 0,
                    right: false
                },
                false,
                false,
                &[]
            )
            .is_empty()
        );
        let uses = v.handle(
            Event::Clicked {
                index: 0,
                right: true,
            },
            false,
            false,
            &[8],
        );
        let Out::Recipes {
            title,
            tabs,
            tab,
            views,
        } = &uses[0]
        else {
            panic!()
        };
        assert_eq!(title, "Uses: Oak Log");
        assert_eq!(
            tabs.iter().map(|t| t.item.as_str()).collect::<Vec<_>>(),
            [STATIONS[0], STATIONS[1]]
        );
        assert_eq!(*tab, 0);
        // Shapeless planks laid out in a 1 x 1 grid; the book doesn't hold it yet.
        assert_eq!(
            (views[0].layout, views[0].width, views[0].note.as_str()),
            (Layout::Grid, 1, "Shapeless")
        );
        assert_eq!(views[0].action, Some(false));
        let furnace = v.handle(Event::Tab(1), false, false, &[]);
        let Out::Recipes { views, .. } = &furnace[0] else {
            panic!()
        };
        assert_eq!(
            (views[0].layout, views[0].note.as_str()),
            (Layout::Cooking, "10 s, 0.15 XP")
        );
        assert_eq!(
            v.handle(Event::Action(0), false, false, &[]),
            [Out::Place(9)]
        );
        // Back with one page shown: no page, the items again.
        let back = v.handle(Event::Back, false, false, &[]);
        assert!(matches!(&back[0], Out::Recipes { views, .. } if views.is_empty()));
    }

    #[test]
    fn keys_and_slot_clicks_open_pages_and_back_returns() {
        let (items, all) = tables();
        let (mut v, _) = Viewer::new(items, all);
        let stick = Stack {
            count: 64,
            ..stack("minecraft:stick")
        };
        let out = v.handle(
            Event::Key {
                uses: false,
                stack: stick,
            },
            false,
            false,
            &[8],
        );
        let Out::Recipes { title, views, .. } = &out[0] else {
            panic!()
        };
        assert_eq!(title, "Recipes: Stick");
        assert_eq!(views[0].action, Some(true));
        let planks = v.handle(
            Event::SlotClicked {
                stack: stack("minecraft:oak_planks"),
                right: false,
            },
            false,
            false,
            &[],
        );
        assert!(matches!(&planks[0], Out::Recipes { title, .. } if title == "Recipes: Oak Planks"));
        let back = v.handle(Event::Back, false, false, &[]);
        assert!(matches!(&back[0], Out::Recipes { title, .. } if title == "Recipes: Stick"));
    }

    #[test]
    fn in_creative_with_the_setting_a_click_gives() {
        let (items, all) = tables();
        let (mut v, _) = Viewer::new(items, all);
        let give = |v: &mut Viewer, right, give, creative| {
            v.handle(Event::Clicked { index: 2, right }, give, creative, &[])
        };
        assert_eq!(
            give(&mut v, false, true, true),
            [Out::Give {
                item: "minecraft:stick".into(),
                count: 1
            }]
        );
        assert_eq!(
            give(&mut v, true, true, true),
            [Out::Give {
                item: "minecraft:stick".into(),
                count: STACK
            }]
        );
        // Off, or not in creative: the page as usual.
        assert!(matches!(
            &give(&mut v, false, false, true)[0],
            Out::Recipes { .. }
        ));
        assert!(matches!(
            &give(&mut v, false, true, false)[0],
            Out::Recipes { .. }
        ));
    }

    #[test]
    fn the_setting_reads_the_webassembly_mods_value() {
        assert!(gives(Some("true".into())));
        assert!(!gives(Some("false".into())));
        assert!(!gives(None));
        assert_eq!(give_label(true), "Click gives items (creative): ON");
    }
}
