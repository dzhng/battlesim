//! An authored scenario's director as a battle runs it: which chapter is
//! open and when each beat fired. The battle evaluates its triggers once a
//! tick (`Battle::direct`); this owns the state and its digest.
use std::collections::BTreeMap;

use contract::command::OrderError;
use contract::director::{Beat, DirectorSetup};
use contract::ids::Tick;

pub struct DirectorRun {
    pub(crate) setup: DirectorSetup,
    /// The open chapter; past the last when every chapter has closed.
    pub(crate) chapter: usize,
    /// When each beat fired, by chapter and beat.
    fired: Vec<Vec<Option<Tick>>>,
    /// Each beat id's place, for `Fired` triggers.
    places: BTreeMap<String, (usize, usize)>,
    /// Orders a beat gave that the battle refused, with the beat's id. Out
    /// of the digest: a test reads it to prove its solution was obeyed.
    pub(crate) refusals: Vec<(String, OrderError)>,
}

impl DirectorRun {
    pub fn new(setup: DirectorSetup) -> Self {
        let fired = setup
            .chapters
            .iter()
            .map(|chapter| vec![None; chapter.beats.len()])
            .collect();
        let places = setup
            .chapters
            .iter()
            .enumerate()
            .flat_map(|(c, chapter)| {
                chapter
                    .beats
                    .iter()
                    .enumerate()
                    .map(move |(b, beat)| (beat.id.clone(), (c, b)))
            })
            .collect();
        Self {
            setup,
            chapter: 0,
            fired,
            places,
            refusals: Vec::new(),
        }
    }

    /// Close the open chapter once all its beats fired before `tick`, so the
    /// next opens on the tick after its last beat.
    pub(crate) fn open_chapter(&mut self, tick: Tick) {
        while self
            .fired
            .get(self.chapter)
            .is_some_and(|beats| beats.iter().all(|fired| fired.is_some_and(|at| at < tick)))
        {
            self.chapter += 1;
        }
    }

    /// The open chapter's beats yet to fire, by their place in it.
    pub(crate) fn waiting(&self) -> Vec<(usize, Beat)> {
        let Some(chapter) = self.setup.chapters.get(self.chapter) else {
            return Vec::new();
        };
        chapter
            .beats
            .iter()
            .enumerate()
            .filter(|(b, _)| self.fired[self.chapter][*b].is_none())
            .map(|(b, beat)| (b, beat.clone()))
            .collect()
    }

    pub(crate) fn fire(&mut self, beat: usize, tick: Tick) {
        self.fired[self.chapter][beat] = Some(tick);
    }

    /// When the beat `id` fired, if it has.
    pub fn fired_at(&self, id: &str) -> Option<Tick> {
        let &(c, b) = self.places.get(id)?;
        self.fired[c][b]
    }

    /// Every beat that has fired, in chapter order, with its tick and the
    /// sides told.
    pub fn fired(&self) -> impl Iterator<Item = (&Beat, Tick)> {
        self.setup
            .chapters
            .iter()
            .zip(&self.fired)
            .flat_map(|(chapter, fired)| {
                chapter
                    .beats
                    .iter()
                    .zip(fired)
                    .filter_map(|(beat, at)| at.map(|at| (beat, at)))
            })
    }

    pub fn chapter(&self) -> usize {
        self.chapter
    }

    pub fn refusals(&self) -> &[(String, OrderError)] {
        &self.refusals
    }

    pub fn digest(&self, d: &mut crate::digest::Digest) {
        d.bytes(b"director").u64(self.chapter as u64);
        for at in self.fired.iter().flatten() {
            d.u64(at.unwrap_or(u64::MAX));
        }
    }
}
