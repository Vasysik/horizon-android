//! The save screen freezes simulation, but keeps pumping window events while
//! snapshots are prepared in small batches and the region workers drain.
use super::{Game, loading_frame};
use horizon_client::{Result, hud::HudBatch, save::SaveDir};
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) struct ClosingSave {
    pub quit: bool,
    close: bool,
    reply: Option<std::sync::mpsc::Sender<String>>,
    started: Instant,
    stage: Stage,
}
enum Stage {
    Generation,
    Chunks(VecDeque<(i32, i32)>),
    Entities(VecDeque<(i32, i32)>),
    Writes,
    Server,
    Quit,
}

// Bound the work in both sparse and block-entity-heavy worlds. A single chunk
// is indivisible; check the time budget after each snapshot, not each batch.
const SNAPSHOTS_PER_FRAME: usize = 16;
const SNAPSHOT_BUDGET: Duration = Duration::from_millis(2);

impl Game<'_> {
    pub(super) fn begin_closing_save(&mut self, quit: bool) -> Result<()> {
        if let Some(save) = &mut self.pending_save {
            save.quit |= quit;
            return Ok(());
        }
        if self.title.is_some() || self.saved_for_exit {
            // A quit from the title still needs the event loop to finish.
            self.pending_save = Some(ClosingSave {
                quit,
                close: true,
                reply: None,
                started: Instant::now(),
                stage: Stage::Quit,
            });
            return Ok(());
        }
        if let Some(integrated) = &self.integrated {
            integrated.request_stop();
            self.pending_save = Some(ClosingSave {
                quit,
                close: true,
                reply: None,
                started: Instant::now(),
                stage: Stage::Server,
            });
        } else if self.remote.is_some() {
            if !quit {
                return self.leave_server(None);
            }
            if let Some(mut remote) = self.remote.take() {
                remote.session.disconnect();
            }
            self.saved_for_exit = true;
            self.pending_save = Some(ClosingSave {
                quit,
                close: true,
                reply: None,
                started: Instant::now(),
                stage: Stage::Quit,
            });
        } else {
            self.close_container();
            self.award_custom_stat("leave_game", 1);
            self.lan_close(false);
            self.free_player_beds()?;
            self.pending_save = Some(ClosingSave {
                quit,
                close: true,
                reply: None,
                started: Instant::now(),
                stage: Stage::Generation,
            });
        }
        self.capture(false);
        Ok(())
    }

    /// The game left the screen (Android: the focus went, or the window), and from now on the
    /// system may end the process without a word: the world is saved where it stands, once each
    /// time the game goes, as Bedrock does, and the log is flushed. It is the autosave's save
    /// (vanilla's when the pause menu opens): the files go to the writer threads and nothing waits
    /// for them, neither the window nor Android's UI thread behind it (a full save waits for the
    /// generation in flight, seconds on a phone). The log says when they are written.
    pub(super) fn save_for_background(&mut self) {
        if !std::mem::replace(&mut self.saved_for_background, true)
            && self.title.is_none()
            && self.pending_save.is_none()
            && self.integrated_starting.is_none()
            && self.pending_entry.is_none()
        {
            if let Some(server) = &self.integrated {
                log::info!("Horizon: saving the world, the game left the screen");
                // Out of sight the world waits as saved; the next frame pauses it or not again.
                if !self.is_published() {
                    server.set_paused(true);
                }
                server.execute(|g| {
                    let started = Instant::now();
                    match g.save_game_with(false) {
                        Ok(()) => {
                            let mut saves = g.levels.parked_saves();
                            saves.push(g.save.clone());
                            log_when_written(saves, started);
                        }
                        Err(e) => log::warn!("Horizon: failed to save the world: {e}"),
                    }
                });
            } else if self.remote.is_none() {
                log::info!("Horizon: saving the world, the game left the screen");
                let started = Instant::now();
                match self.save_game_with(false) {
                    Ok(()) => log_when_written(vec![self.save.clone()], started),
                    Err(e) => log::warn!("Horizon: failed to save the world: {e}"),
                }
            }
        }
        horizon_client::logging::flush();
    }

    /// Control saves acknowledge only after durable completion, while their
    /// snapshot work is spread across event-loop turns too.
    pub(super) fn begin_control_save(
        &mut self,
        all: bool,
        reply: std::sync::mpsc::Sender<String>,
    ) -> Result<()> {
        if self.pending_save.is_some() {
            return Err("a world save is already in progress".into());
        }
        if all {
            self.world
                .modified
                .extend(self.world.chunks.keys().copied());
        }
        self.keys.clear();
        self.physical_keys.clear();
        self.left = false;
        self.right = false;
        self.pending_save = Some(ClosingSave {
            quit: false,
            close: false,
            reply: Some(reply),
            started: Instant::now(),
            stage: Stage::Generation,
        });
        Ok(())
    }

    /// `true` means the window can exit. The session lock stays in Game until
    /// every snapshot has been queued and every writer has synced its files.
    pub(super) fn poll_closing_save(&mut self) -> Result<bool> {
        let Some(mut save) = self.pending_save.take() else {
            return Ok(false);
        };
        let result = (|| -> Result<bool> {
            match &mut save.stage {
                Stage::Quit => Ok(true),
                Stage::Server => {
                    let server = self
                        .integrated
                        .as_ref()
                        .ok_or("the integrated server stopped with an error")?;
                    if !server.stopped() {
                        return Ok(false);
                    }
                    // Drop/join only after completion; leave_server's teardown
                    // can then run without blocking on the server's writes.
                    if save.quit {
                        self.integrated.take().unwrap().stop()?;
                    } else {
                        self.leave_server(None)?;
                    }
                    Ok(true)
                }
                Stage::Generation => {
                    if save.started.elapsed() > Duration::from_secs(30) {
                        return Err("chunk generation did not finish before saving".into());
                    }
                    if self.streaming.poll_save(&mut self.world) {
                        save.stage = Stage::Chunks(self.world.modified.iter().copied().collect());
                    }
                    Ok(false)
                }
                Stage::Chunks(keys) => {
                    let started = Instant::now();
                    for _ in 0..SNAPSHOTS_PER_FRAME {
                        let Some(&key) = keys.front() else { break };
                        let chunks = self.with_entities(self.world.chunks.get(&key));
                        self.save.save_later(
                            &chunks,
                            self.pack.assets.registry,
                            world_biomes!(self),
                        )?;
                        self.world.modified.remove(&key);
                        keys.pop_front();
                        if started.elapsed() >= SNAPSHOT_BUDGET {
                            break;
                        }
                    }
                    if keys.is_empty() {
                        self.spawn_loaded_entities();
                        save.stage = Stage::Entities(self.world.chunks.keys().copied().collect());
                    }
                    Ok(false)
                }
                Stage::Entities(keys) => {
                    let started = Instant::now();
                    for _ in 0..SNAPSHOTS_PER_FRAME {
                        let Some(&key) = keys.front() else { break };
                        let chunks = self.entity_chunks(&HashSet::from([key]), false);
                        self.save.save_entities_later(chunks)?;
                        keys.pop_front();
                        if started.elapsed() >= SNAPSHOT_BUDGET {
                            break;
                        }
                    }
                    if keys.is_empty() {
                        self.prune_entity_memory();
                        self.save_raids();
                        self.save.save_attachments(&self.world.attachments)?;
                        self.save.save_forced_chunks(&self.streaming.forced)?;
                        self.save_game_metadata()?;
                        save.stage = Stage::Writes;
                    }
                    Ok(false)
                }
                Stage::Writes => self.save.writes_finished(),
            }
        })();
        match result {
            Ok(true) => {
                if let Some(reply) = save.reply.take() {
                    let _ = reply.send(serde_json::json!({ "ok": true }).to_string());
                }
                if save.quit && !save.close {
                    // A window close during a control save still has to close
                    // containers and save the resulting inventory changes.
                    self.begin_closing_save(true)?;
                    return Ok(false);
                }
                if save.quit {
                    self.saved_for_exit = true;
                    Ok(true)
                } else {
                    if save.close && self.title.is_none() {
                        self.finish_perf();
                        self.finish_close_world()?;
                    }
                    self.clock.reset();
                    self.previous = Instant::now();
                    Ok(false)
                }
            }
            other => {
                if let Err(error) = &other
                    && let Some(reply) = save.reply.take()
                {
                    let _ =
                        reply.send(serde_json::json!({ "error": error.to_string() }).to_string());
                }
                // Keep ownership (and the lock) on errors too. Native shutdown
                // sees the same durable-write failure instead of declaring success.
                self.pending_save = Some(save);
                other
            }
        }
    }

    pub(super) fn render_closing_save(&mut self) -> Result<()> {
        if matches!(
            self.pending_save.as_ref().map(|save| &save.stage),
            Some(Stage::Quit)
        ) {
            return Ok(());
        }
        // disconnectWithSavingScreen: a GenericMessageScreen, its title and
        // no navigation.
        horizon_client::widgets::begin_screen(|| {
            horizon_client::widgets::ScreenInfo::new(
                "saving",
                super::screen_info::tr("menu.savingLevel", &[]),
            )
            .without_navigation()
        });
        let Some(renderer) = &mut self.renderer else {
            return Ok(());
        };
        let mut hud = HudBatch::new([renderer.config.width, renderer.config.height]);
        let progress = match self.pending_save.as_ref().map(|save| &save.stage) {
            Some(Stage::Chunks(_)) => 0.25,
            Some(Stage::Entities(_)) => 0.5,
            Some(Stage::Writes) => 0.9,
            _ => 0.0,
        };
        horizon_client::level_loading::draw_progress(
            &mut hud,
            &self.gui.atlas,
            "Saving world...",
            progress,
        );
        renderer.render(&loading_frame(&hud))?;
        Ok(())
    }
}

/// Logs, from a thread of its own, once the writer threads have put a save that did not wait on
/// disk: in a bug report it shows whether the process ended before.
fn log_when_written(saves: Vec<Arc<SaveDir>>, started: Instant) {
    let waiter = std::thread::Builder::new()
        .name("Save waiter".into())
        .spawn(move || {
            let written = saves.iter().try_for_each(|save| save.flush());
            horizon_client::save::later::flush();
            match written {
                Ok(()) => log::info!(
                    "Horizon: the world is saved ({:.2} s)",
                    started.elapsed().as_secs_f32()
                ),
                Err(e) => log::warn!("Horizon: failed to save the world: {e}"),
            }
            horizon_client::logging::flush();
        });
    if let Err(e) = waiter {
        log::warn!("Horizon: could not wait for the save: {e}");
    }
}
