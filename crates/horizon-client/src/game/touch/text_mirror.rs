//! The on-screen keyboard's editor on Android (TextMirror.java): an editor the player never sees,
//! holding a copy of the text field shown now, which the keyboard edits as any app's field
//! (suggestions, autocorrect, swipe, its clipboard) instead of sending NativeActivity's key events.
//! Each edit comes back whole (text and selection) and is set on the field; the field's own changes
//! (Tab, the history, a line sent, a hardware keyboard) go back to the editor. The fields it cannot
//! read (a mod's, the recipe book's and the options' searches) keep NativeActivity's keyboard.
// The editor exists on Android only; the rest is the game's side of it.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use super::super::{Game, books::BookScreen};
use horizon_client::{clipboard::FieldId, containers::BlockEntity};
use winit::keyboard::KeyCode;

/// The editor's flags (TextMirror.java): autocorrect and suggestions (messages, books, signs),
/// else none (commands, names, searches); several lines. The keyboard's action key is in the bits
/// from 8 (EditorInfo.IME_ACTION_*).
const SUGGEST: i32 = 1;
const MULTILINE: i32 = 2;
const SEARCH: i32 = 3 << 8;
const SEND: i32 = 4 << 8;
const NEXT: i32 = 5 << 8;
const DONE: i32 = 6 << 8;

/// The field shown now, as the editor mirrors it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Field {
    /// Which field: another one starts afresh in the editor.
    id: FieldId,
    text: String,
    /// The cursor and the selection's other end, in characters; None where the field has none
    /// (it types at its end).
    selection: Option<(usize, usize)>,
    /// The most characters it takes, 0 where only its width limits it.
    max: usize,
    flags: i32,
}

/// What the editor is told this frame.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Push {
    /// Focused with the keyboard up, holding the field; or holding the field's own change. The
    /// cursor and anchor are -1 to keep the editor's own.
    Show(Field, i32, i32, i32),
    Update(Field, i32, i32, i32),
    /// Put away with the keyboard.
    Hide,
}

/// The editor as the game last told it or heard from it.
#[derive(Default)]
pub(super) struct Mirror {
    /// What the editor holds; None while it is put away.
    shown: Option<Field>,
    /// The game's last show or update: an edit made before it reaches the game after it.
    generation: i32,
    /// The show or update the field shown now started with: edits older than it were made on
    /// another field.
    field_generation: i32,
    /// The field the editor was last told about, kept once it is put away: its last edit may
    /// come after (typed just before the back key).
    field: Option<FieldId>,
    /// NativeActivity's keyboard is up, for a field the editor does not mirror.
    pub(super) native: bool,
}

impl Mirror {
    /// The field shown now (None: no editor wanted) against what the editor holds: what to tell
    /// it, if anything.
    pub(super) fn sync(&mut self, field: Option<Field>) -> Option<Push> {
        let Some(field) = field else {
            return self.shown.take().map(|_| Push::Hide);
        };
        let shown = self.shown.as_ref().filter(|s| s.id == field.id);
        if let Some(shown) = shown
            && shown.text == field.text
            && shown.max == field.max
            && shown.flags == field.flags
            && (field.selection.is_none() || field.selection == shown.selection)
        {
            return None;
        }
        let (selection, cursor, anchor) = held(&field, shown);
        let fresh = self.shown.is_none();
        if shown.is_none() {
            self.field_generation = self.generation + 1;
        }
        self.generation += 1;
        let push = if fresh {
            Push::Show(field.clone(), cursor, anchor, self.generation)
        } else {
            Push::Update(field.clone(), cursor, anchor, self.generation)
        };
        self.field = Some(field.id);
        self.shown = Some(Field { selection, ..field });
        Some(push)
    }

    /// An edit the editor made on the update `generation` saw: whether it is for `field`, the
    /// field shown now (older ones were for the one before).
    fn takes(&self, generation: i32, field: Option<FieldId>) -> bool {
        field.is_some() && field == self.field && generation >= self.field_generation
    }

    /// The editor holds `text` after an edit made on the latest update: what it holds now. An
    /// edit made before the latest update reaches the editor after it is set on the field all the
    /// same, and the field goes to the editor again.
    fn edited(&mut self, generation: i32, text: &str, cursor: usize, anchor: usize) {
        if generation == self.generation
            && let Some(shown) = &mut self.shown
        {
            shown.text = text.to_owned();
            shown.selection = Some((cursor, anchor));
        }
    }
}

/// The selection the editor holds once told about `field`, and the cursor and anchor to tell it.
/// A field without a cursor types at its end: the editor's goes there when the text changes, and
/// stays where the player put it otherwise.
fn held(field: &Field, shown: Option<&Field>) -> (Option<(usize, usize)>, i32, i32) {
    let index = |i: usize| i.min(i32::MAX as usize) as i32;
    match field.selection {
        Some((c, a)) => (Some((c, a)), index(c), index(a)),
        None => match shown {
            Some(shown) if shown.text == field.text => (shown.selection, -1, -1),
            _ => {
                let len = field.text.chars().count();
                (Some((len, len)), index(len), index(len))
            }
        },
    }
}

/// Android's KEYCODE_* the editor passes on (TextMirror.key), as winit's.
fn android_key(code: i32) -> Option<KeyCode> {
    Some(match code {
        19 => KeyCode::ArrowUp,
        20 => KeyCode::ArrowDown,
        61 => KeyCode::Tab,
        67 => KeyCode::Backspace,
        92 => KeyCode::PageUp,
        93 => KeyCode::PageDown,
        111 => KeyCode::Escape,
        _ => return None,
    })
}

impl Game<'_> {
    /// The text field shown now as the editor mirrors it; None where there is none, or one the
    /// editor cannot read.
    pub(super) fn mirror_field(&mut self) -> Option<Field> {
        let field = |id, text: &str, selection, max, flags| Field {
            id,
            text: text.to_owned(),
            selection,
            max,
            flags,
        };
        Some(match self.text_context()? {
            "chat" => {
                let screen = self.ui.chat_screen.as_ref()?;
                let input = &screen.input;
                // Autocorrect would wreck a command's words.
                let flags = if input.value.starts_with('/') {
                    SEND
                } else {
                    SUGGEST | SEND
                };
                let selection = Some((input.cursor, input.highlight));
                field(
                    ("chat", 0),
                    &input.value,
                    selection,
                    input.max_length,
                    flags,
                )
            }
            "sign" => {
                let edit = self.sign_edit.as_ref()?;
                let Some(BlockEntity::Sign(sign)) = self.world.block_entities.get(&edit.pos) else {
                    return None;
                };
                let (side, name) = if edit.front {
                    (&sign.front, "sign.front")
                } else {
                    (&sign.back, "sign.back")
                };
                let line = side.lines.get(edit.line)?;
                // Enter goes to the next line.
                field((name, edit.line), line, None, 0, SUGGEST | NEXT)
            }
            "book" => match self.book.as_ref()? {
                // BookSignScreen's title box holds 15.
                BookScreen::Edit {
                    signing: true,
                    title,
                    ..
                } => field(("book.title", 0), title, None, 15, SUGGEST | DONE),
                // A page takes 1024 characters (books.rs), on 14 lines.
                BookScreen::Edit { pages, page, .. } => field(
                    ("book.page", *page),
                    pages.get(*page)?,
                    None,
                    1024,
                    SUGGEST | MULTILINE,
                ),
                BookScreen::View { .. } => return None,
            },
            "command" => {
                let b = self.command_box()?;
                let selection = Some((b.cursor, b.highlight));
                field(("command", 0), &b.value, selection, b.max_length, DONE)
            }
            // AnvilScreen's name box holds 50.
            "anvil" => field(("anvil", 0), &self.station.as_ref()?.name, None, 50, DONE),
            "creative" => field(
                ("creative", 0),
                &self.creative_screen.search.value,
                None,
                super::super::clipboard::CREATIVE_SEARCH_CHARS,
                SEARCH,
            ),
            "title" => {
                let (id, value, max) = self.title.as_mut()?.focused_field()?;
                let value = value.clone();
                field(id, &value, None, max, DONE)
            }
            _ => return None,
        })
    }

    /// The editor's text and selection on the field shown now, through the field's own typing
    /// (its filters and limits hold).
    fn mirror_write(&mut self, text: &str, cursor: usize, anchor: usize) {
        let Some(field) = self.mirror_field() else {
            return;
        };
        match field.id.0 {
            "chat" => self.chat_set_text(text, cursor, anchor),
            "command" => self.command_block_set_text(text, cursor, anchor),
            _ => {
                // A field that types at its end: back to where the two texts part, then the
                // rest typed.
                if field.id.0 == "title"
                    && let Some(t) = self.title.as_mut()
                    && t.has_caret_field()
                {
                    t.field_key(KeyCode::End, false, false);
                }
                let same = field
                    .text
                    .chars()
                    .zip(text.chars())
                    .take_while(|(a, b)| a == b)
                    .count();
                for _ in same..field.text.chars().count() {
                    self.mirror_key(KeyCode::Backspace);
                }
                let rest: String = text.chars().skip(same).collect();
                if field.flags & MULTILINE != 0 {
                    for (i, line) in rest.split('\n').enumerate() {
                        if i > 0 {
                            self.mirror_key(KeyCode::Enter);
                        }
                        self.mirror_type(line);
                    }
                } else {
                    // A line break pasted here goes with the field's filter, as typed.
                    self.mirror_type(&rest);
                }
            }
        }
    }

    /// Text typed into a field that types at its end.
    fn mirror_type(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        match self.text_context() {
            Some("sign") => self.sign_input(None, Some(text)),
            Some("book") => self.book_input(None, Some(text)),
            Some("anvil") => self.station_text(Some(text), false),
            Some("creative") => self.creative_search_edit(Some(text)),
            Some("title") => self.title_edit(Some(text), false),
            _ => {}
        }
    }

    /// A key on the field shown now, as the keyboard's would be: Enter, Backspace, the history's
    /// arrows.
    fn mirror_key(&mut self, key: KeyCode) {
        match self.text_context() {
            Some("chat") => self.chat_key_code(key, None),
            Some("sign") => self.sign_input(Some(key), None),
            Some("book") => self.book_input(Some(key), None),
            Some("command") => match key {
                KeyCode::Enter | KeyCode::NumpadEnter => self.command_block_done(),
                _ => self.command_block_key_code(key, None, None),
            },
            Some("anvil") if key == KeyCode::Backspace => self.station_text(None, true),
            Some("creative") if key == KeyCode::Backspace => self.creative_search_backspace(false),
            Some("title") => match key {
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    if let Some(t) = self.title.as_mut() {
                        t.confirm_edit();
                    }
                }
                KeyCode::Backspace => self.title_edit(None, true),
                KeyCode::Tab => {
                    if let Some(t) = self.title.as_mut() {
                        t.tab();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// The keyboard's action key: the field's Enter. Where Enter leaves the field as it was (a
    /// search, a name), the keyboard goes away, as after Done in any app.
    fn mirror_action(&mut self) {
        let before = self.mirror_field();
        self.mirror_key(KeyCode::Enter);
        if before.is_some() && self.mirror_field() == before {
            self.touch.wanted = false;
        }
    }

    /// An edit from the editor: set on the field when it is the field the edit was made on.
    fn mirror_edit(&mut self, text: &str, cursor: usize, anchor: usize, generation: i32) {
        let field = self.mirror_field().map(|f| f.id);
        if !self.touch.mirror.takes(generation, field) {
            return;
        }
        self.mirror_write(text, cursor, anchor);
        self.touch.mirror.edited(generation, text, cursor, anchor);
    }

    /// The keyboard on Android, each frame once `keyboard` (wanted, the window focused) is
    /// decided: what the editor did applied in order, then the editor shown, told the field's
    /// own changes or put away; or NativeActivity's keyboard for a field it cannot read.
    #[cfg(target_os = "android")]
    pub(super) fn android_keyboard(&mut self, keyboard: bool) {
        for event in crate::android::take_mirror_events() {
            match event {
                crate::android::MirrorEvent::Edit {
                    text,
                    cursor,
                    anchor,
                    generation,
                } => self.mirror_edit(&text, cursor, anchor, generation),
                _ if self.touch.mirror.shown.is_none() => {}
                crate::android::MirrorEvent::Action => self.mirror_action(),
                crate::android::MirrorEvent::Key(111) => {
                    let _ = self.touch_key_tap(KeyCode::Escape);
                }
                crate::android::MirrorEvent::Key(code) => {
                    if let Some(key) = android_key(code) {
                        self.mirror_key(key);
                    }
                }
            }
        }
        let field = if keyboard { self.mirror_field() } else { None };
        // A field the editor cannot read (not one that just went, with the line sent).
        let unread = keyboard && field.is_none() && self.text_context().is_some();
        let push = self.touch.mirror.sync(field);
        match &push {
            Some(Push::Show(f, cursor, anchor, generation))
            | Some(Push::Update(f, cursor, anchor, generation)) => {
                let show = matches!(push, Some(Push::Show(..)));
                let max = f.max.min(i32::MAX as usize) as i32;
                crate::android::text_mirror(
                    show,
                    &f.text,
                    *cursor,
                    *anchor,
                    max,
                    f.flags,
                    *generation,
                );
            }
            Some(Push::Hide) => crate::android::hide_text_mirror(),
            None => {}
        }
        // NativeActivity's keyboard for the rest; not in the frame the editor goes away, whose
        // hiding (on the UI thread) would come after.
        let native = unread && push != Some(Push::Hide);
        if native != self.touch.mirror.native
            && let Some(window) = &self.window
        {
            window.set_ime_allowed(native);
            // winit hides only a keyboard shown implicitly; Android's own call hides any.
            if !native {
                crate::android::hide_keyboard();
            }
            self.touch.mirror.native = native;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(id: usize, text: &str, selection: Option<(usize, usize)>) -> Field {
        Field {
            id: ("test", id),
            text: text.to_owned(),
            selection,
            max: 0,
            flags: SUGGEST,
        }
    }

    #[test]
    fn the_editor_hears_of_the_fields_own_changes_once() {
        let mut m = Mirror::default();
        let f = field(0, "ab", Some((2, 2)));
        assert_eq!(
            m.sync(Some(f.clone())),
            Some(Push::Show(f.clone(), 2, 2, 1))
        );
        assert_eq!(m.sync(Some(f.clone())), None);
        // The editor typed: nothing to tell it back.
        assert!(m.takes(1, Some(f.id)));
        m.edited(1, "abc", 3, 3);
        let typed = field(0, "abc", Some((3, 3)));
        assert_eq!(m.sync(Some(typed.clone())), None);
        // The field changed itself (Tab, the history).
        let tab = field(0, "abcdef", Some((6, 6)));
        assert_eq!(m.sync(Some(tab.clone())), Some(Push::Update(tab, 6, 6, 2)));
        assert_eq!(m.sync(None), Some(Push::Hide));
        assert_eq!(m.sync(None), None);
        // Put away with the back key, the field still there: its last edit still counts; once
        // the field went (the line sent), none.
        assert!(m.takes(2, Some(f.id)));
        assert!(!m.takes(2, None));
    }

    #[test]
    fn an_edit_made_before_the_fields_change_is_kept_and_sent_back() {
        let mut m = Mirror::default();
        m.sync(Some(field(0, "a", Some((1, 1)))));
        // The field changed (update 2) while the player typed on update 1.
        m.sync(Some(field(0, "a1", Some((2, 2)))));
        assert!(m.takes(1, Some(("test", 0))));
        m.edited(1, "ab", 2, 2);
        // The field took the edit: the editor, which got update 2 since, is told again.
        let kept = field(0, "ab", Some((2, 2)));
        assert_eq!(
            m.sync(Some(kept.clone())),
            Some(Push::Update(kept, 2, 2, 3))
        );
    }

    #[test]
    fn edits_for_the_field_before_are_dropped() {
        let mut m = Mirror::default();
        m.sync(Some(field(0, "line one", None)));
        // The next line of the sign: the editor starts again at its end.
        let next = field(1, "two", None);
        assert_eq!(
            m.sync(Some(next.clone())),
            Some(Push::Update(next, 3, 3, 2))
        );
        assert!(!m.takes(1, Some(("test", 1))));
        assert!(m.takes(2, Some(("test", 1))));
        assert!(!m.takes(2, Some(("test", 0))));
    }

    #[test]
    fn a_field_without_a_cursor_leaves_the_editors_where_it_is() {
        let mut m = Mirror::default();
        m.sync(Some(field(0, "abc", None)));
        // The player moved the editor's cursor, then the field's kind changed (no new text).
        m.edited(1, "abc", 1, 1);
        assert_eq!(m.sync(Some(field(0, "abc", None))), None);
        let plain = Field {
            flags: DONE,
            ..field(0, "abc", None)
        };
        assert_eq!(
            m.sync(Some(plain.clone())),
            Some(Push::Update(plain, -1, -1, 2))
        );
        // Text the field changed itself puts it at the end.
        let longer = Field {
            flags: DONE,
            ..field(0, "abcd", None)
        };
        assert_eq!(
            m.sync(Some(longer.clone())),
            Some(Push::Update(longer, 4, 4, 3))
        );
    }

    #[test]
    fn android_keys_the_editor_passes_on() {
        assert_eq!(android_key(67), Some(KeyCode::Backspace));
        assert_eq!(android_key(19), Some(KeyCode::ArrowUp));
        assert_eq!(android_key(29), None);
    }
}
