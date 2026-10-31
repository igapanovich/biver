#![allow(dead_code)]

use itertools::Itertools;
use rand::prelude::*;

pub static EMPTY: &[u8] = &[];

pub static SAMPLE0: &[u8] = include_bytes!("samples/sample-0.kra");
pub static SAMPLE1: &[u8] = include_bytes!("samples/sample-1.kra");
pub static SAMPLE2: &[u8] = include_bytes!("samples/sample-2.kra");
pub static SAMPLE3: &[u8] = include_bytes!("samples/sample-3.kra");
pub static SAMPLE4: &[u8] = include_bytes!("samples/sample-4.kra");
pub static SAMPLE5: &[u8] = include_bytes!("samples/sample-5.kra");
pub static SAMPLE6: &[u8] = include_bytes!("samples/sample-6.kra");
pub static SAMPLE7: &[u8] = include_bytes!("samples/sample-7.kra");
pub static SAMPLE8: &[u8] = include_bytes!("samples/sample-8.kra");
pub static SAMPLE9: &[u8] = include_bytes!("samples/sample-9.kra");

static ALL_SAMPLES: &[&[u8]; 10] = &[
    SAMPLE0, SAMPLE1, SAMPLE2, SAMPLE3, SAMPLE4, SAMPLE5, SAMPLE6, SAMPLE7, SAMPLE8, SAMPLE9,
];

pub struct SampleSeq {
    last_index: usize,
    rng: StdRng,
}

impl SampleSeq {
    pub fn new() -> Self {
        SampleSeq {
            last_index: 0,
            rng: StdRng::seed_from_u64(1),
        }
    }

    pub fn next(&mut self) -> &'static [u8] {
        let indices = (0..ALL_SAMPLES.len())
            .filter(|i| *i != self.last_index)
            .collect_vec();

        let next_index_pos = self.rng.next_u64() % indices.len() as u64;

        let next_index = indices[next_index_pos as usize];
        self.last_index = next_index;

        ALL_SAMPLES[next_index]
    }
}
